use crate::geometry::GeometryVertex;
use crate::scene::{Transform, Vec3};

use super::transforms::{transform_normal, transform_position};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TangentFrame {
    pub(super) tangent: Vec3,
    pub(super) handedness: f32,
}

/// Generate per-vertex tangent frames for a glTF mesh using the canonical
/// MikkTSpace algorithm. Phase 1E plan-line-856 closure: production
/// arbitrary-glTF tangent generation now goes through the same algorithm
/// the parity oracle uses, so authored normal maps from any glTF source
/// resolve to the tangent basis MikkTSpace defines.
pub(super) fn accumulate_vertex_tangents(
    vertices: &[GeometryVertex],
    indices: &[u32],
    tex_coords0: &[[f32; 2]],
    transform: Transform,
    origin_shift: Vec3,
) -> Vec<TangentFrame> {
    let face_count = indices.len() / 3;
    let mut adapter = MikktspaceAdapter {
        positions: vertices
            .iter()
            .map(|vertex| transform_position(vertex.position, transform, origin_shift))
            .collect(),
        normals: vertices
            .iter()
            .map(|vertex| transform_normal(vertex.normal, transform))
            .collect(),
        tex_coords0,
        indices,
        output: vec![
            TangentFrame {
                tangent: Vec3::new(1.0, 0.0, 0.0),
                handedness: 1.0,
            };
            vertices.len()
        ],
        face_count,
    };
    if face_count > 0 {
        let result = bevy_mikktspace::generate_tangents(&mut adapter);
        debug_assert!(
            result.is_ok(),
            "bevy_mikktspace exposes an uninhabited tangent-generation error"
        );
    }
    // Re-orthogonalize against the per-vertex normal so a downstream
    // tangent-space basis stays orthonormal even when the geometry's
    // authored normals do not exactly match MikkTSpace's per-face normal
    // averaging. Authored-tangent path applies the same step; preserving
    // it for generated tangents keeps both paths under the same
    // contract.
    for (frame, vertex) in adapter.output.iter_mut().zip(vertices) {
        let normal = transform_normal(vertex.normal, transform);
        let orthogonal = subtract_vec3(
            frame.tangent,
            scale_vec3(normal, dot_vec3(frame.tangent, normal)),
        );
        frame.tangent = normalize_or(orthogonal, fallback_tangent(normal));
    }
    adapter.output
}

struct MikktspaceAdapter<'a> {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    tex_coords0: &'a [[f32; 2]],
    indices: &'a [u32],
    output: Vec<TangentFrame>,
    face_count: usize,
}

impl<'a> MikktspaceAdapter<'a> {
    fn vertex_index(&self, face: usize, vert: usize) -> usize {
        self.indices[face * 3 + vert] as usize
    }
}

impl<'a> bevy_mikktspace::Geometry for MikktspaceAdapter<'a> {
    fn num_faces(&self) -> usize {
        self.face_count
    }

    fn num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        let position = self.positions[self.vertex_index(face, vert)];
        [position.x, position.y, position.z]
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        let normal = self.normals[self.vertex_index(face, vert)];
        [normal.x, normal.y, normal.z]
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        self.tex_coords0[self.vertex_index(face, vert)]
    }

    fn set_tangent(
        &mut self,
        tangent_space: Option<bevy_mikktspace::TangentSpace>,
        face: usize,
        vert: usize,
    ) {
        let Some(tangent_space) = tangent_space else {
            return;
        };
        let tangent = tangent_space.tangent_encoded();
        let index = self.vertex_index(face, vert);
        self.output[index] = TangentFrame {
            tangent: Vec3::new(tangent[0], tangent[1], tangent[2]),
            handedness: if tangent[3] < 0.0 { -1.0 } else { 1.0 },
        };
    }
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

#[cfg(test)]
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
    fn accumulated_vertex_tangents_resolve_shared_triangle_through_mikktspace() {
        // Phase 1E plan-line-856 closure: production tangents now go
        // through the MikkTSpace algorithm instead of the prior in-tree
        // weighted-average. Vertex 0 is shared by two triangles with
        // different UV mappings (the first puts the U axis along +X, the
        // second along +Y). MikkTSpace resolves shared vertices per UV
        // island rather than averaging across face boundaries, so the
        // returned tangent matches one of the two per-face tangents
        // (specifically the +X mapping from the first face) and stays
        // orthogonal to the vertex normal. This is the canonical glTF
        // tangent-space contract.
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

        assert_vec3_near(tangents[0].tangent, Vec3::new(1.0, 0.0, 0.0));
        assert!(
            dot_vec3(tangents[0].tangent, vertices[0].normal).abs() < 0.0001,
            "MikkTSpace tangent must stay orthogonal to the vertex normal"
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

    #[test]
    fn accumulated_vertex_tangents_generate_stable_indexed_quad_basis() {
        // Two-triangle quad with shared corners: the simplest non-trivial
        // mesh that exercises shared-vertex tangent generation. Every
        // vertex sees identical (position, normal, uv) across both
        // incident triangles, so the MikkTSpace basis must be the +U
        // axis with preserved handedness at every corner.
        let vertices = [
            GeometryVertex {
                position: Vec3::new(-1.0, -1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(1.0, -1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(1.0, 1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(-1.0, 1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ];
        let tex_coords = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let indices = [0u32, 1, 2, 0, 2, 3];

        let in_tree = accumulate_vertex_tangents(
            &vertices,
            &indices,
            &tex_coords,
            Transform::IDENTITY,
            Vec3::ZERO,
        );

        assert_eq!(in_tree.len(), vertices.len());
        for (vertex_index, frame) in in_tree.iter().enumerate() {
            assert_vec3_near(frame.tangent, Vec3::new(1.0, 0.0, 0.0));
            assert_eq!(
                frame.handedness, 1.0,
                "vertex {vertex_index}: indexed quad handedness must be preserved",
            );
        }
    }

    #[test]
    fn accumulated_vertex_tangents_fall_back_for_degenerate_uvs() {
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
        let collapsed_uvs = [[0.5, 0.5], [0.5, 0.5], [0.5, 0.5]];

        let tangents = accumulate_vertex_tangents(
            &vertices,
            &[0, 1, 2],
            &collapsed_uvs,
            Transform::IDENTITY,
            Vec3::ZERO,
        );

        for frame in tangents {
            assert!(
                frame.tangent.x.is_finite()
                    && frame.tangent.y.is_finite()
                    && frame.tangent.z.is_finite(),
                "degenerate UV fallback tangent must be finite"
            );
            assert!(
                dot_vec3(frame.tangent, vertices[0].normal).abs() < 0.0001,
                "degenerate UV fallback tangent must stay orthogonal to the normal"
            );
        }
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
