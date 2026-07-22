use super::*;

#[test]
fn extended_primitives_have_deterministic_counts_bounds_and_normals() {
    let cases = [
        ("box", GeometryDesc::box_xyz(0.20, 0.12, 0.16), 24, 36),
        ("sphere", GeometryDesc::sphere(0.11, 8, 4), 45, 192),
        ("cylinder", GeometryDesc::cylinder(0.10, 0.22, 12), 52, 144),
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

#[test]
fn cone_faces_are_outward_from_computed_geometry_truth() {
    let geometry = GeometryDesc::cone(0.10, 0.22, 12);
    let half_height = 0.11;
    let mut lateral = 0;
    let mut bottom_cap = 0;

    for (triangle_index, triangle) in geometry.indices().chunks_exact(3).enumerate() {
        let positions = [triangle[0], triangle[1], triangle[2]]
            .map(|index| geometry.vertices()[index as usize].position);
        let face_normal = computed_face_normal(positions)
            .unwrap_or_else(|| panic!("cone triangle {triangle_index} must be nondegenerate"));
        let centroid = scale(
            add(add(positions[0], positions[1]), positions[2]),
            1.0 / 3.0,
        );
        if positions
            .iter()
            .all(|position| (position.y + half_height).abs() <= 1.0e-6)
        {
            bottom_cap += 1;
            assert!(
                face_normal.y < -0.99,
                "cone bottom-cap triangle {triangle_index} must face down: {face_normal:?}"
            );
        } else {
            lateral += 1;
            let outward = Vec3::new(centroid.x, 0.0, centroid.z);
            assert!(
                dot(face_normal, outward) > 0.0 && face_normal.y > 0.0,
                "cone lateral triangle {triangle_index} must face radially outward and up: face={face_normal:?}, centroid={centroid:?}"
            );
        }
    }

    assert_eq!(lateral, 12);
    assert_eq!(bottom_cap, 12);
}

#[test]
fn wedge_faces_are_outward_from_computed_geometry_truth() {
    let geometry = GeometryDesc::wedge(0.20, 0.12, 0.16);
    // The triangular-prism volume centroid is the centroid of its Y/Z
    // cross-section, not the bounds center: (-h,-d), (-h,+d), (+h,+d).
    let volume_centroid = Vec3::new(0.0, -0.06 / 3.0, 0.08 / 3.0);
    let mut nondegenerate = 0;
    for (triangle_index, triangle) in geometry.indices().chunks_exact(3).enumerate() {
        let positions = [triangle[0], triangle[1], triangle[2]]
            .map(|index| geometry.vertices()[index as usize].position);
        let Some(face_normal) = computed_face_normal(positions) else {
            continue;
        };
        nondegenerate += 1;
        let centroid = scale(
            add(add(positions[0], positions[1]), positions[2]),
            1.0 / 3.0,
        );
        assert!(
            dot(face_normal, sub(centroid, volume_centroid)) > 0.0,
            "wedge triangle {triangle_index} must point away from its volume center: face={face_normal:?}, centroid={centroid:?}"
        );
    }
    assert_eq!(nondegenerate, 8);
}

#[test]
fn cone_and_wedge_dimensions_do_not_encode_transform_sign_or_scale() {
    let cone = GeometryDesc::cone(0.10, 0.22, 12);
    let negative_cone = GeometryDesc::cone(-0.10, -0.22, 12);
    assert_eq!(cone.vertices(), negative_cone.vertices());
    assert_eq!(cone.indices(), negative_cone.indices());

    let wedge = GeometryDesc::wedge(0.20, 0.12, 0.16);
    let negative_wedge = GeometryDesc::wedge(-0.20, -0.12, -0.16);
    assert_eq!(wedge.vertices(), negative_wedge.vertices());
    assert_eq!(wedge.indices(), negative_wedge.indices());
}

fn computed_face_normal(positions: [Vec3; 3]) -> Option<Vec3> {
    let normal = cross(
        sub(positions[1], positions[0]),
        sub(positions[2], positions[0]),
    );
    let length = (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z).sqrt();
    (length > 1.0e-6).then(|| scale(normal, length.recip()))
}

fn dot(lhs: Vec3, rhs: Vec3) -> f32 {
    lhs.x * rhs.x + lhs.y * rhs.y + lhs.z * rhs.z
}
