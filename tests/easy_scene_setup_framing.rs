use scena::{
    Aabb, Assets, FramingOptions, GeometryDesc, GridFloorOptions, LookupError, MaterialDesc,
    MaterialKind, NodeKind, OrbitControls, OrthographicCamera, PerspectiveCamera, Scene, Transform,
    Vec3,
};

fn viewport() -> (u32, u32) {
    (420, 720)
}

fn desktop_viewport() -> (u32, u32) {
    (1280, 720)
}

fn wide_bounds() -> Aabb {
    Aabb::new(Vec3::new(-3.0, -0.45, -0.25), Vec3::new(3.0, 0.45, 0.25))
}

fn tall_bounds() -> Aabb {
    Aabb::new(Vec3::new(-0.35, -3.0, -0.25), Vec3::new(0.35, 3.0, 0.25))
}

fn corners(bounds: Aabb) -> [Vec3; 8] {
    [
        Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ]
}

#[test]
fn frame_bounds_projects_wide_object_inside_portrait_viewport() {
    let (width, height) = viewport();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");

    let bounds = wide_bounds();
    let outcome = scene
        .frame_bounds(
            camera,
            bounds,
            FramingOptions::new()
                .view_direction(Vec3::new(0.8, 0.35, 0.7))
                .fill(0.70)
                .margin_px(8.0)
                .viewport(width, height),
        )
        .expect("framing succeeds");

    assert!(outcome.projected_rect.min_x >= 8.0, "{outcome:?}");
    assert!(outcome.projected_rect.min_y >= 8.0, "{outcome:?}");
    assert!(
        outcome.projected_rect.max_x <= width as f32 - 8.0,
        "{outcome:?}"
    );
    assert!(
        outcome.projected_rect.max_y <= height as f32 - 8.0,
        "{outcome:?}"
    );
    assert!(
        (0.66..=0.72).contains(&outcome.projected_rect.fill_fraction(width, height)),
        "{outcome:?}"
    );

    for corner in corners(bounds) {
        let projected = scene
            .project_world_point(camera, corner, width, height)
            .expect("projection succeeds")
            .expect("corner is in front of the camera");
        assert!(
            projected.x >= 0.0 && projected.x <= width as f32,
            "{projected:?}"
        );
        assert!(
            projected.y >= 0.0 && projected.y <= height as f32,
            "{projected:?}"
        );
    }
}

#[test]
fn frame_bounds_projects_wide_object_inside_desktop_viewport() {
    let (width, height) = desktop_viewport();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");

    let bounds = wide_bounds();
    let outcome = scene
        .frame_bounds(
            camera,
            bounds,
            FramingOptions::new()
                .front()
                .fill(0.70)
                .margin_px(16.0)
                .viewport(width, height),
        )
        .expect("desktop framing succeeds");

    assert!(outcome.projected_rect.min_x >= 16.0, "{outcome:?}");
    assert!(outcome.projected_rect.min_y >= 16.0, "{outcome:?}");
    assert!(
        outcome.projected_rect.max_x <= width as f32 - 16.0,
        "{outcome:?}"
    );
    assert!(
        outcome.projected_rect.max_y <= height as f32 - 16.0,
        "{outcome:?}"
    );
    assert!(
        (0.66..=0.72).contains(&outcome.projected_rect.fill_fraction(width, height)),
        "{outcome:?}"
    );
}

#[test]
fn frame_bounds_projects_tall_object_inside_portrait_viewport() {
    let (width, height) = viewport();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");

    let bounds = tall_bounds();
    let outcome = scene
        .frame_bounds(
            camera,
            bounds,
            FramingOptions::new()
                .front()
                .fill(0.70)
                .margin_px(12.0)
                .viewport(width, height),
        )
        .expect("portrait framing succeeds");

    assert!(outcome.projected_rect.min_y >= 12.0, "{outcome:?}");
    assert!(
        outcome.projected_rect.max_y <= height as f32 - 12.0,
        "{outcome:?}"
    );
    assert!(
        (0.66..=0.72).contains(&outcome.projected_rect.fill_fraction(width, height)),
        "{outcome:?}"
    );
}

#[test]
fn frame_bounds_offsets_target_for_off_center_bounds() {
    let (width, height) = desktop_viewport();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::default(),
        )
        .expect("camera inserts");
    let bounds = Aabb::new(Vec3::new(6.0, -0.2, -0.3), Vec3::new(8.0, 0.6, 0.3));

    let outcome = scene
        .frame_bounds(
            camera,
            bounds,
            FramingOptions::new()
                .look_from(Vec3::new(0.35, 0.25, 1.0))
                .fill(0.70)
                .viewport(width, height),
        )
        .expect("off-center framing succeeds");

    assert!(
        (outcome.target.x - bounds.center().x).abs() < 0.5,
        "target should be derived from object bounds, not world origin: {outcome:?}"
    );
    assert!(
        (outcome.projected_rect.center_x() - width as f32 * 0.5).abs() < width as f32 * 0.02,
        "{outcome:?}"
    );
    assert!(
        (outcome.projected_rect.center_y() - height as f32 * 0.5).abs() < height as f32 * 0.02,
        "{outcome:?}"
    );
}

#[test]
fn frame_bounds_rejects_empty_bounds_without_silent_fallback() {
    let (width, height) = viewport();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::default(),
        )
        .expect("camera inserts");

    let err = scene
        .frame_bounds(
            camera,
            Aabb::new(Vec3::ZERO, Vec3::ZERO),
            FramingOptions::new().viewport(width, height),
        )
        .expect_err("empty bounds are invalid");

    assert!(matches!(err, LookupError::ImportHasNoBounds));
}

#[test]
fn frame_bounds_rejects_orthographic_until_supported() {
    let (width, height) = viewport();
    let mut scene = Scene::new();
    let camera = scene
        .add_orthographic_camera(
            scene.root(),
            OrthographicCamera::default(),
            Transform::default(),
        )
        .expect("orthographic camera inserts");

    let err = scene
        .frame_bounds(
            camera,
            wide_bounds(),
            FramingOptions::new().viewport(width, height),
        )
        .expect_err("orthographic frame_bounds is explicitly unsupported for this patch");

    assert!(matches!(
        err,
        LookupError::UnsupportedCameraType {
            operation: "frame_bounds",
            ..
        }
    ));
}

#[test]
fn project_world_point_tracks_transformed_label_anchor() {
    let (width, height) = desktop_viewport();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::default(),
        )
        .expect("camera inserts");
    scene
        .frame_bounds(
            camera,
            Aabb::new(Vec3::new(-2.5, -0.5, -0.5), Vec3::new(2.5, 0.5, 0.5)),
            FramingOptions::new().front().viewport(width, height),
        )
        .expect("framing succeeds");

    let left = scene
        .project_world_point(camera, Vec3::new(-1.5, 0.0, 0.0), width, height)
        .expect("projection succeeds")
        .expect("left anchor visible");
    let right = scene
        .project_world_point(camera, Vec3::new(1.5, 0.0, 0.0), width, height)
        .expect("projection succeeds")
        .expect("right anchor visible");

    assert!(
        right.x > left.x + width as f32 * 0.1,
        "projected labels must be driven by world positions, not static CSS: left={left:?} right={right:?}"
    );
}

#[test]
fn frame_bounds_can_run_before_prepare_and_marks_transform_dirty() {
    let (width, height) = viewport();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::default(),
        )
        .expect("camera inserts");
    let before = scene.dirty_state();

    scene
        .frame_bounds(
            camera,
            wide_bounds(),
            FramingOptions::new().viewport(width, height),
        )
        .expect("framing before prepare/render succeeds");

    let after = scene.dirty_state();
    assert!(
        after.transform_revision > before.transform_revision,
        "framing must dirty camera transforms without prepare/render"
    );
}

#[test]
fn orbit_controls_can_adopt_frame_bounds_target_as_pivot() {
    let (width, height) = viewport();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::default(),
        )
        .expect("camera inserts");

    let outcome = scene
        .frame_bounds(
            camera,
            wide_bounds(),
            FramingOptions::new()
                .view_direction(Vec3::new(0.4, 0.3, 1.0))
                .viewport(width, height),
        )
        .expect("framing succeeds");

    let controls = OrbitControls::new(Vec3::ZERO, 1.0).focus_on_framing(outcome);
    assert_eq!(controls.target(), outcome.target);
    assert_eq!(controls.distance(), outcome.distance);
    assert!(
        (controls.yaw_radians() - outcome.yaw_radians).abs() < 1e-5,
        "focus_on_framing must adopt the framed yaw; demo code must not patch it with literal angles"
    );
    assert!(
        (controls.pitch_radians() - outcome.pitch_radians).abs() < 1e-5,
        "focus_on_framing must adopt the framed pitch; demo code must not patch it with literal angles"
    );

    let mut controlled_scene = Scene::new();
    let controlled_camera = controlled_scene
        .add_perspective_camera(
            controlled_scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::default(),
        )
        .expect("camera inserts");
    controls
        .apply_to_scene(&mut controlled_scene, controlled_camera)
        .expect("focused controls apply");
    let controlled = controlled_scene
        .project_world_point(controlled_camera, outcome.target, width, height)
        .expect("projection succeeds")
        .expect("target is visible");
    assert!(
        (controlled.x - width as f32 * 0.5).abs() < width as f32 * 0.02,
        "adopted controls should preserve the framed pose without post-focus angle constants: {controlled:?}"
    );
}

#[test]
fn frame_bounds_rejects_invalid_options_without_silent_fallback() {
    let (width, height) = viewport();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::default(),
        )
        .expect("camera inserts");

    let err = scene
        .frame_bounds(
            camera,
            wide_bounds(),
            FramingOptions::new().fill(1.2).viewport(width, height),
        )
        .expect_err("fill > 1 is invalid");

    assert!(matches!(
        err,
        LookupError::InvalidFramingOption { field: "fill", .. }
    ));

    let err = scene
        .frame_bounds(
            camera,
            wide_bounds(),
            FramingOptions::new()
                .view_direction(Vec3::ZERO)
                .viewport(width, height),
        )
        .expect_err("zero view direction is invalid");

    assert!(matches!(
        err,
        LookupError::InvalidFramingOption {
            field: "view_direction",
            ..
        }
    ));
}

#[test]
fn aabb_union_is_public_and_reusable_for_multi_state_framing() {
    let left = Aabb::new(Vec3::new(-2.0, -1.0, -0.5), Vec3::new(-1.0, 0.25, 0.1));
    let right = Aabb::new(Vec3::new(0.5, -0.25, -1.0), Vec3::new(2.5, 1.5, 0.4));

    let union = left.union(right);

    assert_eq!(union.min, Vec3::new(-2.0, -1.0, -1.0));
    assert_eq!(union.max, Vec3::new(2.5, 1.5, 0.4));
}

#[test]
fn frame_bounds_keeps_default_far_plane_for_reflective_environment_safety() {
    let (width, height) = viewport();
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::default(),
        )
        .expect("camera inserts");

    scene
        .frame_bounds(
            camera,
            wide_bounds(),
            FramingOptions::new().viewport(width, height),
        )
        .expect("framing succeeds");

    let scena::Camera::Perspective(camera) = scene.camera(camera).expect("camera exists") else {
        panic!("perspective camera");
    };
    assert!(
        camera.far >= 1000.0,
        "default framing must not over-tighten far clip and risk IBL/reflection regressions"
    );
}

#[test]
fn frame_bounds_keeps_default_far_plane_for_transmission_control_asset() {
    let assets = Assets::new();
    let asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/khronos/TransmissionTest/TransmissionTest.glb"),
    )
    .expect("TransmissionTest loads as an optional-transmission control");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&asset)
        .expect("TransmissionTest instantiates");
    let bounds = import
        .bounds_world(&scene)
        .expect("TransmissionTest has renderable bounds");
    let (width, height) = desktop_viewport();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::default(),
        )
        .expect("camera inserts");

    scene
        .frame_bounds(
            camera,
            bounds,
            FramingOptions::new().isometric().viewport(width, height),
        )
        .expect("framing succeeds for optional-transmission asset");

    let scena::Camera::Perspective(camera) = scene.camera(camera).expect("camera exists") else {
        panic!("perspective camera");
    };
    assert!(
        camera.far >= 1000.0,
        "default framing must not tighten far clip for optional-transmission PBR control assets"
    );
}

#[test]
fn grid_floor_is_matte_bounds_derived_and_lies_on_floor_plane() {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let bounds = Aabb::new(Vec3::new(-1.4, 0.0, -0.35), Vec3::new(1.2, 0.8, 0.42));

    let floor = scene
        .add_grid_floor(
            &assets,
            GridFloorOptions::new()
                .under_bounds(bounds)
                .floor_y(0.0)
                .padding(0.4),
        )
        .expect("grid floor inserts");

    assert_eq!(floor.bounds.min.y, 0.0);
    assert_eq!(floor.bounds.max.y, 0.0);
    assert!(floor.bounds.min.x <= bounds.min.x - 0.19, "{floor:?}");
    assert!(floor.bounds.max.x >= bounds.max.x + 0.19, "{floor:?}");

    let slab_node = scene.node(floor.slab).expect("slab node exists");
    let NodeKind::Mesh(slab_mesh) = slab_node.kind() else {
        panic!("slab is a mesh");
    };
    let slab_material = assets
        .material(slab_mesh.material())
        .expect("slab material exists");
    assert_eq!(slab_material.kind(), MaterialKind::PbrMetallicRoughness);
    assert_eq!(slab_material.metallic_factor(), 0.0);
    assert!(slab_material.roughness_factor() >= 0.9);

    let grid_node = scene.node(floor.grid).expect("grid node exists");
    let NodeKind::Mesh(grid_mesh) = grid_node.kind() else {
        panic!("grid is a mesh");
    };
    let grid_geometry = assets
        .geometry(grid_mesh.geometry())
        .expect("grid geometry");
    assert!(
        grid_geometry
            .vertices()
            .iter()
            .all(|vertex| vertex.position.y.abs() < 1e-6),
        "all grid vertices must lie on the floor plane"
    );
}

#[test]
fn bounds_for_transforms_unions_discrete_replay_poses() {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::default());
    let node = scene.mesh(geometry, material).add().expect("mesh inserts");

    let bounds = scene
        .bounds_for_transforms(
            node,
            &[
                Transform::at(Vec3::new(-2.0, 0.0, 0.0)),
                Transform::at(Vec3::new(2.0, 0.0, 0.0)),
            ],
            &assets,
        )
        .expect("bounds union resolves");

    assert!(bounds.min.x <= -2.5, "{bounds:?}");
    assert!(bounds.max.x >= 2.5, "{bounds:?}");
    assert!(bounds.min.y <= -0.2, "{bounds:?}");
    assert!(bounds.max.y >= 0.2, "{bounds:?}");
}
