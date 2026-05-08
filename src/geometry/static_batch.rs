use crate::scene::{Quat, Transform, Vec3};

use super::{GeometryDesc, GeometryVertex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticBatchReport {
    source_vertices: usize,
    source_indices: usize,
    instance_count: usize,
    output_vertices: usize,
    output_indices: usize,
}

impl GeometryDesc {
    pub fn static_batch(
        source: &GeometryDesc,
        transforms: impl IntoIterator<Item = Transform>,
    ) -> Self {
        let transforms = transforms.into_iter().collect::<Vec<_>>();
        if transforms.is_empty() {
            return source.clone();
        }

        let mut vertices = Vec::with_capacity(source.vertices.len() * transforms.len());
        let mut indices = Vec::with_capacity(source.indices.len() * transforms.len());
        let mut vertex_colors = Vec::with_capacity(source.vertex_colors.len() * transforms.len());
        let mut tex_coords0 = Vec::with_capacity(source.tex_coords0.len() * transforms.len());
        let mut tangents = source
            .tangents
            .as_ref()
            .map(|source_tangents| Vec::with_capacity(source_tangents.len() * transforms.len()));
        for transform in transforms {
            let base = vertices.len() as u32;
            vertices.extend(source.vertices.iter().map(|vertex| GeometryVertex {
                position: transform_point(vertex.position, transform),
                normal: rotate_vec3(transform.rotation, vertex.normal),
            }));
            indices.extend(source.indices.iter().map(|index| base + *index));
            vertex_colors.extend(source.vertex_colors.iter().copied());
            tex_coords0.extend(source.tex_coords0.iter().copied());
            if let (Some(target), Some(source_tangents)) = (&mut tangents, &source.tangents) {
                target.extend(source_tangents.iter().map(|tangent| {
                    let rotated = rotate_vec3(
                        transform.rotation,
                        Vec3::new(tangent[0], tangent[1], tangent[2]),
                    );
                    [rotated.x, rotated.y, rotated.z, tangent[3]]
                }));
            }
        }

        let geometry = Self::try_new_with_vertex_colors_and_tex_coords(
            source.topology,
            vertices,
            indices,
            vertex_colors,
            tex_coords0,
        )
        .expect(
            "static batching preserves valid source geometry topology, indices, and attributes",
        );
        match tangents {
            Some(tangents) => geometry
                .with_tangents(tangents)
                .expect("static batching preserves valid authored tangents"),
            None => geometry,
        }
    }

    pub fn static_batch_report(source: &GeometryDesc, instance_count: usize) -> StaticBatchReport {
        let instance_count = instance_count.max(1);
        StaticBatchReport {
            source_vertices: source.vertices.len(),
            source_indices: source.indices.len(),
            instance_count,
            output_vertices: source.vertices.len() * instance_count,
            output_indices: source.indices.len() * instance_count,
        }
    }
}

impl StaticBatchReport {
    pub const fn source_vertices(self) -> usize {
        self.source_vertices
    }

    pub const fn source_indices(self) -> usize {
        self.source_indices
    }

    pub const fn instance_count(self) -> usize {
        self.instance_count
    }

    pub const fn output_vertices(self) -> usize {
        self.output_vertices
    }

    pub const fn output_indices(self) -> usize {
        self.output_indices
    }

    pub const fn requires_prepare_after_rebuild(self) -> bool {
        true
    }

    pub const fn picking_debug_instances(self) -> usize {
        self.instance_count
    }
}

fn transform_point(point: Vec3, transform: Transform) -> Vec3 {
    let scaled = Vec3::new(
        point.x * transform.scale.x,
        point.y * transform.scale.y,
        point.z * transform.scale.z,
    );
    let rotated = rotate_vec3(transform.rotation, scaled);
    Vec3::new(
        rotated.x + transform.translation.x,
        rotated.y + transform.translation.y,
        rotated.z + transform.translation.z,
    )
}

fn rotate_vec3(rotation: Quat, vector: Vec3) -> Vec3 {
    let length_squared = rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return vector;
    }
    let inverse_length = length_squared.sqrt().recip();
    let qx = rotation.x * inverse_length;
    let qy = rotation.y * inverse_length;
    let qz = rotation.z * inverse_length;
    let qw = rotation.w * inverse_length;
    let tx = 2.0 * (qy * vector.z - qz * vector.y);
    let ty = 2.0 * (qz * vector.x - qx * vector.z);
    let tz = 2.0 * (qx * vector.y - qy * vector.x);
    Vec3::new(
        vector.x + qw * tx + (qy * tz - qz * ty),
        vector.y + qw * ty + (qz * tx - qx * tz),
        vector.z + qw * tz + (qx * ty - qy * tx),
    )
}
