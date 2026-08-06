#![cfg(not(target_arch = "wasm32"))]

use scena::{
    Aabb, Assets, Color, EnvironmentDesc, GeometryDesc, MaterialDesc, PerspectiveCamera,
    ReflectionProbe, Renderer, Scene, Transform, Vec3,
};

#[test]
fn headless_gpu_shades_assigned_metal_from_local_probe() {
    let assets = Assets::new();
    let global_environment = assets.create_environment(
        EnvironmentDesc::from_cubemap_radiance(
            "scena://generated/reflection-probe/global-blue",
            16,
            std::array::from_fn(|_| vec![[0.02, 0.05, 4.0]; 16 * 16]),
        )
        .expect("global cubemap is valid"),
    );
    let local_environment = assets.create_environment(
        EnvironmentDesc::from_cubemap_radiance(
            "scena://generated/reflection-probe/local-red",
            16,
            std::array::from_fn(|_| vec![[4.0, 0.03, 0.02]; 16 * 16]),
        )
        .expect("local cubemap is valid"),
    );
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.9, 1.0, 0.2));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::WHITE,
        1.0,
        0.08,
    ));

    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    let local = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-0.65, 0.0, 0.0)))
        .add()
        .expect("local-probe mesh inserts");
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(0.65, 0.0, 0.0)))
        .add()
        .expect("global-environment mesh inserts");
    scene
        .add_reflection_probe(
            ReflectionProbe::new(Aabb::new(
                Vec3::new(-1.2, -0.7, -0.4),
                Vec3::new(-0.1, 0.7, 0.4),
            ))
            .with_capture_position(Vec3::new(-0.65, 0.0, 0.0))
            .with_resolution(16)
            .with_environment(local_environment)
            .assign_node(local),
        )
        .expect("local probe inserts");

    let Ok(mut renderer) = Renderer::headless_gpu(160, 90) else {
        return;
    };
    renderer.set_environment(global_environment);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("probe scene prepares");
    renderer.render_active(&scene).expect("probe scene renders");

    let frame = renderer.frame_rgba8();
    let left = dominant_color(frame, 160, 8..80, 12..78);
    let right = dominant_color(frame, 160, 80..152, 12..78);
    assert!(
        left[0].saturating_sub(left[2]) > 50_000,
        "the assigned left metal must sample the red local probe, got RGB sums {left:?}",
    );
    assert!(
        right[2].saturating_sub(right[0]) > 50_000,
        "the unassigned right metal must retain the blue global environment, got RGB sums {right:?}",
    );
}

fn dominant_color(
    frame: &[u8],
    width: u32,
    x_range: std::ops::Range<u32>,
    y_range: std::ops::Range<u32>,
) -> [u64; 3] {
    let mut sums = [0_u64; 3];
    for y in y_range {
        for x in x_range.clone() {
            let offset = ((y * width + x) * 4) as usize;
            sums[0] += u64::from(frame[offset]);
            sums[1] += u64::from(frame[offset + 1]);
            sums[2] += u64::from(frame[offset + 2]);
        }
    }
    sums
}
