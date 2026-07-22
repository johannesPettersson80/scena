use scena::{
    Assets, Camera, Color, FramingOptions, GeometryDesc, MaterialDesc, PerspectiveCamera, Renderer,
    Scene, Transform, Vec3,
};

#[test]
fn visible_bounds_centering_moves_content_center_not_node_origin() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(2.0, 2.0, 2.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let group = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(100.0, 0.0, 0.0)))
        .expect("group inserts");
    scene
        .mesh(geometry, material)
        .parent(group)
        .transform(Transform::at(Vec3::new(10.0, 0.0, 0.0)))
        .add()
        .expect("offset mesh inserts");

    scene
        .center_visible_bounds_on(group, &assets, Vec3::ZERO)
        .expect("visible content centers");

    assert!(
        scene
            .world_transform(group)
            .expect("group transform")
            .translation
            .abs_diff_eq(Vec3::new(-10.0, 0.0, 0.0), 1.0e-5),
        "the group origin must retain its offset from the visible center"
    );
    let centered = scene
        .node_world_bounds(group, &assets)
        .expect("bounds lookup")
        .expect("visible bounds");
    assert!(centered.center().abs_diff_eq(Vec3::ZERO, 1.0e-5));
}

#[test]
fn framing_options_exclude_hidden_and_inspection_helpers_unless_requested() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("subject inserts");
    let helper = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(50.0, 0.0, 0.0)))
        .add()
        .expect("helper inserts");
    scene
        .add_tag(helper, "scena:inspection:helper")
        .expect("helper tag inserts");
    let hidden = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-100.0, 0.0, 0.0)))
        .add()
        .expect("hidden mesh inserts");
    scene.set_visible(hidden, false).expect("hidden mesh hides");
    let camera = scene.add_default_camera().expect("camera inserts");

    let subject = scene
        .frame_all_with_assets_and_options(
            camera,
            &assets,
            FramingOptions::new().viewport(300, 900),
        )
        .expect("subject-only framing succeeds");
    let with_helper = scene
        .frame_all_with_assets_and_options(
            camera,
            &assets,
            FramingOptions::new()
                .viewport(300, 900)
                .include_helpers(true),
        )
        .expect("helper-inclusive framing succeeds");

    assert!(subject.target.x.abs() < 1.0, "{subject:?}");
    assert!(with_helper.target.x > 20.0, "{with_helper:?}");
    assert!(
        with_helper.target.x < 30.0,
        "hidden content must stay excluded"
    );
}

#[test]
fn aggregate_framing_contains_multiple_imports() {
    let assets = Assets::new();
    let asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("fixture glTF loads");
    let mut scene = Scene::new();
    let left = scene.instantiate(&asset).expect("left import instantiates");
    let right = scene
        .instantiate(&asset)
        .expect("right import instantiates");
    scene
        .set_transform(left.roots()[0], Transform::at(Vec3::new(-8.0, 0.0, 0.0)))
        .expect("left import moves");
    scene
        .set_transform(right.roots()[0], Transform::at(Vec3::new(12.0, 0.0, 0.0)))
        .expect("right import moves");
    let left_bounds = scene
        .node_world_bounds(left.roots()[0], &assets)
        .expect("left bounds lookup")
        .expect("left import has bounds");
    let right_bounds = scene
        .node_world_bounds(right.roots()[0], &assets)
        .expect("right bounds lookup")
        .expect("right import has bounds");
    let expected_center = Vec3::new(
        (left_bounds.min.x.min(right_bounds.min.x) + left_bounds.max.x.max(right_bounds.max.x))
            * 0.5,
        (left_bounds.min.y.min(right_bounds.min.y) + left_bounds.max.y.max(right_bounds.max.y))
            * 0.5,
        (left_bounds.min.z.min(right_bounds.min.z) + left_bounds.max.z.max(right_bounds.max.z))
            * 0.5,
    );
    let camera = scene.add_default_camera().expect("camera inserts");

    let framing = scene
        .frame_all_with_assets_and_options(
            camera,
            &assets,
            FramingOptions::new().viewport(1_000, 500),
        )
        .expect("both imports frame together");

    assert!(framing.target.abs_diff_eq(expected_center, 1.0e-5));
    assert!(framing.projected_rect.min_x >= 0.0, "{framing:?}");
    assert!(framing.projected_rect.max_x <= 1_000.0, "{framing:?}");
}

#[test]
fn target_viewport_aspect_drives_projection_and_cpu_pixels() {
    const WIDTH: u32 = 240;
    const HEIGHT: u32 = 720;
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(4.0, 0.8, 0.6));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 160, 240)));
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(7.0, 2.0, 0.0)))
        .add()
        .expect("off-origin mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(16.0 / 9.0),
            Transform::IDENTITY,
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");

    let framing = scene
        .frame_all_with_assets_and_options(
            camera,
            &assets,
            FramingOptions::new()
                .three_quarter_front_right()
                .fill(0.70)
                .margin_px(12.0)
                .tighten_depth_range(true)
                .viewport(WIDTH, HEIGHT),
        )
        .expect("portrait framing succeeds");
    let Camera::Perspective(camera_desc) = scene.camera(camera).expect("camera exists") else {
        panic!("framing fixture uses perspective camera");
    };
    assert!((camera_desc.aspect - WIDTH as f32 / HEIGHT as f32).abs() < 1.0e-6);
    assert!(camera_desc.near > 0.0);
    assert!(camera_desc.far > camera_desc.near);
    assert!(framing.projected_rect.min_x >= 12.0, "{framing:?}");
    assert!(framing.projected_rect.min_y >= 12.0, "{framing:?}");
    assert!(
        framing.projected_rect.max_x <= WIDTH as f32 - 12.0,
        "{framing:?}"
    );
    assert!(
        framing.projected_rect.max_y <= HEIGHT as f32 - 12.0,
        "{framing:?}"
    );

    let mut renderer = Renderer::headless(WIDTH, HEIGHT).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("portrait scene prepares");
    renderer
        .render_active(&scene)
        .expect("portrait scene renders");
    let (min_x, min_y, max_x, max_y) = nonblack_bounds(renderer.frame_rgba8(), WIDTH, HEIGHT)
        .expect("framed subject renders visible pixels");
    assert!(min_x > 0 && max_x < WIDTH - 1);
    assert!(min_y > 0 && max_y < HEIGHT - 1);
    assert!(((min_x + max_x) as f32 * 0.5 - WIDTH as f32 * 0.5).abs() < WIDTH as f32 * 0.12);
    assert!(((min_y + max_y) as f32 * 0.5 - HEIGHT as f32 * 0.5).abs() < HEIGHT as f32 * 0.12);
}

fn nonblack_bounds(frame: &[u8], width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for y in 0..height {
        for x in 0..width {
            let offset = ((y * width + x) * 4) as usize;
            if frame[offset..offset + 3].iter().any(|channel| *channel > 8) {
                bounds = Some(match bounds {
                    Some((min_x, min_y, max_x, max_y)) => {
                        (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
                    }
                    None => (x, y, x, y),
                });
            }
        }
    }
    bounds
}
