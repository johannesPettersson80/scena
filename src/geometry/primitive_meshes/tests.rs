use super::*;

#[test]
fn extended_primitives_have_deterministic_counts_bounds_and_normals() {
    let cases = [
        ("box", GeometryDesc::box_xyz(0.20, 0.12, 0.16), 24, 36),
        ("sphere", GeometryDesc::sphere(0.11, 8, 4), 45, 192),
        ("cylinder", GeometryDesc::cylinder(0.10, 0.22, 12), 50, 144),
        ("plane", GeometryDesc::plane(0.20, 0.16), 4, 6),
        ("cone", GeometryDesc::cone(0.10, 0.22, 12), 49, 72),
        ("torus", GeometryDesc::torus(0.11, 0.03, 12, 6), 91, 432),
        ("disc", GeometryDesc::disc(0.12, 16), 17, 48),
        ("wedge", GeometryDesc::wedge(0.20, 0.12, 0.16), 18, 24),
        (
            "beveled_box",
            GeometryDesc::box_xyz_with_bevel(0.20, 0.12, 0.16, 0.01),
            96,
            132,
        ),
        (
            "beveled_cylinder",
            GeometryDesc::cylinder_with_bevel(0.10, 0.22, 12, 0.01),
            216,
            288,
        ),
    ];
    for (name, geometry, vertex_count, index_count) in cases {
        assert_eq!(geometry.topology(), GeometryTopology::Triangles, "{name}");
        assert_eq!(geometry.vertices().len(), vertex_count, "{name}");
        assert_eq!(geometry.indices().len(), index_count, "{name}");
        let bounds = geometry.bounds();
        for value in [
            bounds.min.x,
            bounds.min.y,
            bounds.min.z,
            bounds.max.x,
            bounds.max.y,
            bounds.max.z,
        ] {
            assert!(value.is_finite(), "{name} bound component must be finite");
        }
        assert!(
            bounds.max.x > bounds.min.x || bounds.max.z > bounds.min.z,
            "{name} should have a measurable silhouette in the ground plane"
        );
        assert!(
            geometry.vertices().iter().any(|vertex| {
                let normal = vertex.normal;
                let length =
                    (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z).sqrt();
                length > 0.9 && length < 1.1
            }),
            "{name} should carry renderable normals"
        );
    }
}

#[test]
fn built_in_triangle_primitives_are_wound_against_vertex_normals() {
    let cases = [
        ("box", GeometryDesc::box_xyz(0.20, 0.12, 0.16)),
        ("sphere", GeometryDesc::sphere(0.11, 8, 4)),
        ("cylinder", GeometryDesc::cylinder(0.10, 0.22, 12)),
        ("plane", GeometryDesc::plane(0.20, 0.16)),
        ("cone", GeometryDesc::cone(0.10, 0.22, 12)),
        ("torus", GeometryDesc::torus(0.11, 0.03, 12, 6)),
        ("disc", GeometryDesc::disc(0.12, 16)),
        ("wedge", GeometryDesc::wedge(0.20, 0.12, 0.16)),
        (
            "beveled_box",
            GeometryDesc::box_xyz_with_bevel(0.20, 0.12, 0.16, 0.01),
        ),
        (
            "beveled_cylinder",
            GeometryDesc::cylinder_with_bevel(0.10, 0.22, 12, 0.01),
        ),
    ];

    for (name, geometry) in cases {
        assert_eq!(geometry.topology(), GeometryTopology::Triangles, "{name}");
        for (triangle_index, triangle) in geometry.indices().chunks_exact(3).enumerate() {
            let a = geometry.vertices()[triangle[0] as usize];
            let b = geometry.vertices()[triangle[1] as usize];
            let c = geometry.vertices()[triangle[2] as usize];
            let face_normal = cross(sub(b.position, a.position), sub(c.position, a.position));
            let face_length = (face_normal.x * face_normal.x
                + face_normal.y * face_normal.y
                + face_normal.z * face_normal.z)
                .sqrt();
            if face_length <= 1.0e-6 {
                continue;
            }
            let face_normal = scale(face_normal, 1.0 / face_length);
            let vertex_normal = normalize(add(add(a.normal, b.normal), c.normal));
            let alignment = face_normal.x * vertex_normal.x
                + face_normal.y * vertex_normal.y
                + face_normal.z * vertex_normal.z;
            assert!(
                alignment > 0.0,
                "{name} triangle {triangle_index} is wound against its vertex normals: alignment={alignment}"
            );
        }
    }
}
