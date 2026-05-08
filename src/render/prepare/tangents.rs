use crate::geometry::GeometryVertex;
use crate::scene::{Transform, Vec3};

use super::transforms::{transform_normal, transform_position};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TangentFrame {
    pub(super) tangent: Vec3,
    pub(super) handedness: f32,
}

pub(super) fn accumulate_vertex_tangents(
    vertices: &[GeometryVertex],
    indices: &[u32],
    tex_coords0: &[[f32; 2]],
    transform: Transform,
    origin_shift: Vec3,
) -> Vec<TangentFrame> {
    let mut accumulated_tangent = vec![Vec3::ZERO; vertices.len()];
    let mut accumulated_bitangent = vec![Vec3::ZERO; vertices.len()];
    for triangle in indices.chunks_exact(3) {
        let ia = triangle[0] as usize;
        let ib = triangle[1] as usize;
        let ic = triangle[2] as usize;
        let Some((raw_tangent, raw_bitangent)) = raw_triangle_tangent_frame(
            transform_position(vertices[ia].position, transform, origin_shift),
            transform_position(vertices[ib].position, transform, origin_shift),
            transform_position(vertices[ic].position, transform, origin_shift),
            tex_coords0[ia],
            tex_coords0[ib],
            tex_coords0[ic],
        ) else {
            continue;
        };
        for index in [ia, ib, ic] {
            accumulated_tangent[index] = add_vec3(accumulated_tangent[index], raw_tangent);
            accumulated_bitangent[index] = add_vec3(accumulated_bitangent[index], raw_bitangent);
        }
    }
    vertices
        .iter()
        .zip(accumulated_tangent)
        .zip(accumulated_bitangent)
        .map(|((vertex, raw_tangent), raw_bitangent)| {
            let normal = transform_normal(vertex.normal, transform);
            let orthogonal = subtract_vec3(
                raw_tangent,
                scale_vec3(normal, dot_vec3(raw_tangent, normal)),
            );
            let tangent = normalize_or(orthogonal, fallback_tangent(normal));
            let handedness = if dot_vec3(cross_vec3(normal, tangent), raw_bitangent) < 0.0 {
                -1.0
            } else {
                1.0
            };
            TangentFrame {
                tangent,
                handedness,
            }
        })
        .collect()
}

pub(super) fn authored_vertex_tangents(
    tangents: Option<&[[f32; 4]]>,
    vertices: &[GeometryVertex],
    transform: Transform,
) -> Option<Vec<TangentFrame>> {
    let tangents = tangents?;
    Some(
        vertices
            .iter()
            .zip(tangents.iter().copied())
            .map(|(vertex, tangent)| {
                let normal = transform_normal(vertex.normal, transform);
                let transformed =
                    transform_normal(Vec3::new(tangent[0], tangent[1], tangent[2]), transform);
                let orthogonal = subtract_vec3(
                    transformed,
                    scale_vec3(normal, dot_vec3(transformed, normal)),
                );
                TangentFrame {
                    tangent: normalize_or(orthogonal, fallback_tangent(normal)),
                    handedness: if tangent[3] < 0.0 { -1.0 } else { 1.0 },
                }
            })
            .collect(),
    )
}

#[cfg(test)]
fn triangle_tangent(
    position_a: Vec3,
    position_b: Vec3,
    position_c: Vec3,
    uv_a: [f32; 2],
    uv_b: [f32; 2],
    uv_c: [f32; 2],
    normal: Vec3,
) -> Vec3 {
    let raw = raw_triangle_tangent_frame(position_a, position_b, position_c, uv_a, uv_b, uv_c)
        .map(|(tangent, _)| tangent)
        .unwrap_or_else(|| fallback_tangent(normal));
    let orthogonal = subtract_vec3(raw, scale_vec3(normal, dot_vec3(raw, normal)));
    normalize_or(orthogonal, fallback_tangent(normal))
}

fn raw_triangle_tangent_frame(
    position_a: Vec3,
    position_b: Vec3,
    position_c: Vec3,
    uv_a: [f32; 2],
    uv_b: [f32; 2],
    uv_c: [f32; 2],
) -> Option<(Vec3, Vec3)> {
    let edge_ab = subtract_vec3(position_b, position_a);
    let edge_ac = subtract_vec3(position_c, position_a);
    let delta_uv_ab = [uv_b[0] - uv_a[0], uv_b[1] - uv_a[1]];
    let delta_uv_ac = [uv_c[0] - uv_a[0], uv_c[1] - uv_a[1]];
    let determinant = delta_uv_ab[0] * delta_uv_ac[1] - delta_uv_ac[0] * delta_uv_ab[1];
    if determinant.abs() <= f32::EPSILON || !determinant.is_finite() {
        return None;
    }
    let inverse = determinant.recip();
    let tangent = scale_vec3(
        subtract_vec3(
            scale_vec3(edge_ab, delta_uv_ac[1]),
            scale_vec3(edge_ac, delta_uv_ab[1]),
        ),
        inverse,
    );
    let bitangent = scale_vec3(
        subtract_vec3(
            scale_vec3(edge_ac, delta_uv_ab[0]),
            scale_vec3(edge_ab, delta_uv_ac[0]),
        ),
        inverse,
    );
    Some((tangent, bitangent))
}

fn fallback_tangent(normal: Vec3) -> Vec3 {
    let axis = if normal.x.abs() < 0.9 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::new(0.0, 1.0, 0.0)
    };
    normalize_or(cross_vec3(axis, normal), Vec3::new(1.0, 0.0, 0.0))
}

fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

fn dot_vec3(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn cross_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(
        left.y * right.z - left.z * right.y,
        left.z * right.x - left.x * right.z,
        left.x * right.y - left.y * right.x,
    )
}

fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let length = dot_vec3(value, value).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        fallback
    } else {
        scale_vec3(value, length.recip())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_triangle_tangent_follows_texcoord_u_axis() {
        let tangent = triangle_tangent(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            [0.0, 0.0],
            [1.0, 0.0],
            [0.0, 1.0],
            Vec3::new(0.0, 0.0, 1.0),
        );

        assert_vec3_near(tangent, Vec3::new(0.0, 1.0, 0.0));
        assert!(
            dot_vec3(tangent, Vec3::new(0.0, 0.0, 1.0)).abs() < 0.0001,
            "generated tangent must stay orthogonal to the geometric normal"
        );
    }

    #[test]
    fn generated_triangle_tangent_falls_back_for_degenerate_uvs() {
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let tangent = triangle_tangent(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            [0.5, 0.5],
            [0.5, 0.5],
            [0.5, 0.5],
            normal,
        );

        assert!(
            tangent.x.is_finite() && tangent.y.is_finite() && tangent.z.is_finite(),
            "degenerate UV fallback tangent must be finite"
        );
        assert!(
            dot_vec3(tangent, normal).abs() < 0.0001,
            "degenerate UV fallback tangent must stay orthogonal to the normal"
        );
    }

    #[test]
    fn accumulated_vertex_tangents_average_shared_triangle_contributions() {
        let vertices = [
            GeometryVertex {
                position: Vec3::new(0.0, 0.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(0.0, 1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(1.0, 0.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(1.0, 0.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(0.0, -1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ];
        let tex_coords = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 0.0], [0.0, 1.0]];

        let tangents = accumulate_vertex_tangents(
            &vertices,
            &[0, 1, 2, 0, 3, 4],
            &tex_coords,
            Transform::IDENTITY,
            Vec3::ZERO,
        );

        let diagonal = 0.5_f32.sqrt();
        assert_vec3_near(tangents[0].tangent, Vec3::new(diagonal, diagonal, 0.0));
        assert!(
            dot_vec3(tangents[0].tangent, vertices[0].normal).abs() < 0.0001,
            "accumulated tangent must stay orthogonal to the vertex normal"
        );
    }

    #[test]
    fn accumulated_vertex_tangents_preserve_mirrored_uv_handedness() {
        let vertices = [
            GeometryVertex {
                position: Vec3::new(0.0, 0.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(1.0, 0.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(0.0, 1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ];
        let mirrored_uvs = [[0.0, 0.0], [1.0, 0.0], [0.0, -1.0]];

        let tangents = accumulate_vertex_tangents(
            &vertices,
            &[0, 1, 2],
            &mirrored_uvs,
            Transform::IDENTITY,
            Vec3::ZERO,
        );

        assert_vec3_near(tangents[0].tangent, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(
            tangents[0].handedness, -1.0,
            "mirrored UV islands must flip tangent-space bitangent handedness"
        );
    }

    #[test]
    fn authored_vertex_tangents_preserve_handedness_and_orthogonalize() {
        let vertices = [GeometryVertex {
            position: Vec3::ZERO,
            normal: Vec3::new(0.0, 0.0, 1.0),
        }];
        let authored = [[1.0, 0.0, 0.5, -1.0]];

        let tangents = authored_vertex_tangents(Some(&authored), &vertices, Transform::IDENTITY)
            .expect("authored tangents are present");

        assert_vec3_near(tangents[0].tangent, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(tangents[0].handedness, -1.0);
        assert!(
            dot_vec3(tangents[0].tangent, vertices[0].normal).abs() < 0.0001,
            "authored tangent must be re-orthogonalized against the prepared normal"
        );
    }

    fn assert_vec3_near(actual: Vec3, expected: Vec3) {
        assert!(
            (actual.x - expected.x).abs() < 0.0001
                && (actual.y - expected.y).abs() < 0.0001
                && (actual.z - expected.z).abs() < 0.0001,
            "expected {expected:?}, got {actual:?}"
        );
    }
}
