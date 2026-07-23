use scena::{
    Angle, Assets, Camera, Color, GeometryDesc, MaterialDesc, OrthographicCamera,
    PerspectiveCamera, RenderMode, Renderer, RendererOptions, Scene, Transform,
};

#[test]
fn invalid_perspective_projection_is_rejected_before_it_enters_the_scene() {
    for camera in [
        PerspectiveCamera {
            vertical_fov: Angle::from_degrees(0.0),
            ..PerspectiveCamera::default()
        },
        PerspectiveCamera {
            vertical_fov: Angle::from_degrees(180.0),
            ..PerspectiveCamera::default()
        },
        PerspectiveCamera {
            vertical_fov: Angle::from_radians(f32::NAN),
            ..PerspectiveCamera::default()
        },
        PerspectiveCamera {
            aspect: -1.0,
            ..PerspectiveCamera::default()
        },
        PerspectiveCamera {
            aspect: f32::INFINITY,
            ..PerspectiveCamera::default()
        },
        PerspectiveCamera {
            near: 0.0,
            ..PerspectiveCamera::default()
        },
        PerspectiveCamera {
            far: f32::NAN,
            ..PerspectiveCamera::default()
        },
        PerspectiveCamera {
            near: 10.0,
            far: 1.0,
            ..PerspectiveCamera::default()
        },
    ] {
        let mut scene = Scene::new();
        let error = scene
            .add_perspective_camera(scene.root(), camera, Transform::IDENTITY)
            .expect_err("invalid projection must fail closed");
        assert!(
            error.to_string().contains("camera projection"),
            "error must identify the projection contract: {error}",
        );
    }

    let mut scene = Scene::new();
    scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::IDENTITY,
        )
        .expect("zero aspect is the documented target-aspect sentinel");
}

#[test]
fn invalid_orthographic_projection_and_public_camera_updates_fail_closed() {
    for camera in [
        OrthographicCamera {
            left: 1.0,
            right: 1.0,
            ..OrthographicCamera::default()
        },
        OrthographicCamera {
            bottom: 2.0,
            top: -2.0,
            ..OrthographicCamera::default()
        },
        OrthographicCamera {
            near: 4.0,
            far: 4.0,
            ..OrthographicCamera::default()
        },
        OrthographicCamera {
            top: f32::INFINITY,
            ..OrthographicCamera::default()
        },
    ] {
        let mut scene = Scene::new();
        assert!(
            scene
                .add_orthographic_camera(scene.root(), camera, Transform::IDENTITY)
                .is_err(),
            "invalid orthographic projection must fail closed: {camera:?}",
        );
    }

    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::IDENTITY,
        )
        .expect("valid camera inserts");
    let before = scene.dirty_state();
    let original = scene.camera(camera).cloned().expect("camera exists");
    assert!(
        scene
            .set_camera(
                camera,
                Camera::Perspective(PerspectiveCamera {
                    near: f32::NAN,
                    ..PerspectiveCamera::standard()
                }),
            )
            .is_err(),
    );
    assert_eq!(scene.camera(camera), Some(&original));
    assert_eq!(scene.dirty_state().camera_revision, before.camera_revision);

    scene
        .set_camera(camera, original.clone())
        .expect("no-op update validates");
    assert_eq!(scene.dirty_state().camera_revision, before.camera_revision);
    scene
        .set_camera(
            camera,
            Camera::Perspective(PerspectiveCamera::standard().with_fov_degrees(55.0)),
        )
        .expect("changed update validates");
    assert_eq!(
        scene.dirty_state().camera_revision,
        before.camera_revision + 1
    );
}

#[test]
fn every_intrinsic_only_camera_change_invalidates_on_change_rendering() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(scena::Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let mut renderer = Renderer::headless_with_options(
        32,
        32,
        RendererOptions::default().with_render_mode(RenderMode::OnChange),
    )
    .expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial scene prepares");
    renderer
        .render(&scene, camera)
        .expect("initial frame renders");
    assert!(
        renderer
            .render(&scene, camera)
            .expect("unchanged frame evaluates")
            .skipped,
    );

    let mut descriptor = PerspectiveCamera::standard();
    type CameraMutation = (&'static str, fn(&mut PerspectiveCamera));
    let mutations: [CameraMutation; 4] = [
        ("aspect", |camera: &mut PerspectiveCamera| {
            camera.aspect = 16.0 / 9.0
        }),
        ("fov", |camera: &mut PerspectiveCamera| {
            camera.vertical_fov = Angle::from_degrees(80.0)
        }),
        ("near", |camera: &mut PerspectiveCamera| camera.near = 0.2),
        ("far", |camera: &mut PerspectiveCamera| camera.far = 2_000.0),
    ];
    for (name, mutate) in mutations {
        mutate(&mut descriptor);
        scene
            .set_camera(camera, Camera::Perspective(descriptor))
            .unwrap_or_else(|error| panic!("{name} update validates: {error}"));
        assert!(
            renderer.render(&scene, camera).is_err(),
            "{name}-only changes must invalidate camera-dependent prepared state",
        );
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .unwrap_or_else(|error| panic!("{name} change prepares: {error}"));
        let outcome = renderer
            .render(&scene, camera)
            .unwrap_or_else(|error| panic!("{name} change renders: {error}"));
        assert!(!outcome.skipped, "{name}-only change must produce a frame");
        assert!(
            renderer
                .render(&scene, camera)
                .unwrap_or_else(|error| panic!("unchanged {name} frame evaluates: {error}"))
                .skipped,
            "unchanged {name} descriptor must return to the on-change fast path",
        );
    }
}
