use scena::{EnvironmentDesc, EnvironmentSourceKind};

#[test]
fn local_probe_cubemap_radiance_preserves_six_linear_faces() {
    let resolution = 2;
    let faces: [Vec<[f32; 3]>; 6] = std::array::from_fn(|face| {
        (0..resolution * resolution)
            .map(|pixel| {
                [
                    face as f32 + 0.25,
                    pixel as f32 + 0.5,
                    face as f32 * 10.0 + pixel as f32,
                ]
            })
            .collect()
    });

    let environment = EnvironmentDesc::from_cubemap_radiance(
        "scena://generated/reflection-probe/7",
        resolution,
        faces.clone(),
    )
    .expect("valid probe radiance creates an environment");

    assert_eq!(
        environment.source_kind(),
        EnvironmentSourceKind::LocalReflectionProbe
    );
    assert_eq!(
        environment.source_dimensions(),
        Some((resolution, resolution))
    );
    assert_eq!(environment.cubemap_resolution(), resolution);

    let decoded = environment
        .cubemap_faces()
        .expect("probe environment retains cubemap faces")
        .build_face_pixels_rgba32f();
    for (face_index, decoded_face) in decoded.iter().enumerate() {
        for (pixel_index, expected) in faces[face_index].iter().enumerate() {
            let offset = pixel_index * 4;
            assert_eq!(
                &decoded_face[offset..offset + 3],
                expected,
                "face {face_index} pixel {pixel_index} must retain linear radiance",
            );
            assert_eq!(decoded_face[offset + 3], 1.0);
        }
    }
}
