use scena::{
    FlyControls, FollowControls, GeometryDesc, MaterialDesc, PerspectiveCamera, Scene, Transform,
    Vec3,
};

fn assert_close(left: f32, right: f32) {
    assert!(
        (left - right).abs() <= 1.0e-4,
        "expected {left} to be close to {right}"
    );
}

#[test]
fn follow_controls_track_node_with_named_offset() {
    let assets = scena::Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::matte(scena::Color::BLUE));
    let mut scene = Scene::new();
    let target = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(2.0, 0.5, -1.0)))
        .add()
        .expect("target mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 5.0)),
        )
        .expect("camera inserts");

    FollowControls::behind_and_above(3.0, 1.25)
        .apply_to_scene(&mut scene, camera, target)
        .expect("follow controls apply");

    let camera_node = scene.camera_node(camera).expect("camera node exists");
    let camera_world = scene.world_transform(camera_node).expect("camera world");

    assert_close(camera_world.translation.x, 2.0);
    assert_close(camera_world.translation.y, 1.75);
    assert_close(camera_world.translation.z, 2.0);
}

#[test]
fn fly_controls_move_in_camera_local_axes_and_apply_to_scene() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::ZERO),
        )
        .expect("camera inserts");
    let mut controls = FlyControls::new(Vec3::ZERO)
        .with_yaw_pitch_degrees(90.0, 0.0)
        .with_move_speed(2.0);

    controls.move_local(1.0, 0.5, 0.25, 2.0);
    controls
        .apply_to_scene(&mut scene, camera)
        .expect("fly controls apply");

    let camera_node = scene.camera_node(camera).expect("camera node exists");
    let camera_world = scene.world_transform(camera_node).expect("camera world");

    assert_close(camera_world.translation.x, 4.0);
    assert_close(camera_world.translation.y, 1.0);
    assert_close(camera_world.translation.z, -2.0);
}
