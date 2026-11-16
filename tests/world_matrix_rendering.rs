//! Rendered proof that both backends place geometry with the world matrix.
//!
//! Each case renders through the CPU rasterizer and, when a headless adapter
//! is available, through the GPU path. Placement is measured as the pixel
//! bounding box of a bar against the box corners carried through
//! `Scene::world_matrix`. Facing is measured by rendering mirrored, single-sided
//! geometry next to an unmirrored twin that occupies the same world space.
#![cfg(not(target_arch = "wasm32"))]

#[allow(dead_code)]
mod support;

use scena::{
    Assets, CameraKey, Color, DirectionalLight, GeometryDesc, Mat4, MaterialDesc, NodeKey,
    OrthographicCamera, PerspectiveCamera, Quat, Renderer, Scene, Transform, Vec3,
};

const SIZE: u32 = 200;
/// World units visible on each side of the origin.
const HALF_EXTENT: f32 = 2.0;
const BAR_HALF_SIZE: Vec3 = Vec3::new(0.5, 0.1, 0.1);
/// Half a pixel of rasterization coverage on either side of an edge, plus one
/// pixel of slack for edge rules that differ between the two rasterizers.
const BOUNDS_TOLERANCE_PX: f32 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PixelBounds {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

impl PixelBounds {
    fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    fn height(self) -> f32 {
        self.max_y - self.min_y
    }
}

struct BarHierarchy {
    name: &'static str,
    /// Local transforms from the scene root down to the bar mesh node.
    chain: &'static [Transform],
}

const QUARTER_TURN_Z: Quat = Quat::from_xyzw(
    0.0,
    0.0,
    std::f32::consts::FRAC_1_SQRT_2,
    std::f32::consts::FRAC_1_SQRT_2,
);
/// 30 degrees about Z.
const TWELFTH_TURN_Z: Quat = Quat::from_xyzw(0.0, 0.0, 0.258_819_04, 0.965_925_8);
/// 45 degrees about X.
const EIGHTH_TURN_X: Quat = Quat::from_xyzw(0.382_683_43, 0.0, 0.0, 0.923_879_5);

const BAR_HIERARCHIES: &[BarHierarchy] = &[
    BarHierarchy {
        name: "non-uniform parent scale, rotated child",
        chain: &[
            Transform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::new(2.0, 1.0, 1.0),
            },
            Transform {
                translation: Vec3::ZERO,
                rotation: QUARTER_TURN_Z,
                scale: Vec3::ONE,
            },
        ],
    },
    BarHierarchy {
        name: "mirrored parent, rotated and translated child, offset mesh",
        chain: &[
            Transform {
                translation: Vec3::new(0.3, 0.1, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(-1.0, 1.0, 1.0),
            },
            Transform {
                translation: Vec3::new(0.4, 0.2, 0.0),
                rotation: TWELFTH_TURN_Z,
                scale: Vec3::ONE,
            },
            Transform {
                translation: Vec3::new(0.6, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
        ],
    },
    BarHierarchy {
        name: "three sheared levels",
        chain: &[
            Transform {
                translation: Vec3::new(-0.2, 0.3, 0.0),
                rotation: TWELFTH_TURN_Z,
                scale: Vec3::new(1.5, 0.6, 1.0),
            },
            Transform {
                translation: Vec3::new(0.3, -0.4, 0.0),
                rotation: QUARTER_TURN_Z,
                scale: Vec3::new(0.7, 1.8, 1.0),
            },
            Transform {
                translation: Vec3::new(0.2, 0.1, 0.0),
                rotation: EIGHTH_TURN_X,
                scale: Vec3::new(1.0, 2.0, 0.5),
            },
        ],
    },
];

#[test]
fn cpu_places_bars_with_the_world_matrix() {
    let renderer = Renderer::headless(SIZE, SIZE).expect("CPU renderer builds");
    assert_bars_follow_the_world_matrix(Backend::Cpu, renderer);
}

#[test]
fn gpu_places_bars_with_the_world_matrix() {
    let Some(renderer) = headless_gpu_renderer("gpu_places_bars_with_the_world_matrix") else {
        return;
    };
    assert_bars_follow_the_world_matrix(Backend::Gpu, renderer);
}

#[test]
fn cpu_renders_the_reported_bar_at_its_true_aspect() {
    let renderer = Renderer::headless(SIZE, SIZE).expect("CPU renderer builds");
    assert_reported_bar_aspect(Backend::Cpu, renderer);
}

#[test]
fn gpu_renders_the_reported_bar_at_its_true_aspect() {
    let Some(renderer) = headless_gpu_renderer("gpu_renders_the_reported_bar_at_its_true_aspect")
    else {
        return;
    };
    assert_reported_bar_aspect(Backend::Gpu, renderer);
}

#[test]
fn cpu_keeps_mirrored_single_sided_geometry_facing_outward() {
    assert_mirrored_geometry_matches_its_twin(Backend::Cpu, || {
        Some(Renderer::headless(SIZE, SIZE).expect("CPU renderer builds"))
    });
}

#[test]
fn gpu_keeps_mirrored_single_sided_geometry_facing_outward() {
    if headless_gpu_renderer("gpu_keeps_mirrored_single_sided_geometry_facing_outward").is_none() {
        return;
    }
    assert_mirrored_geometry_matches_its_twin(Backend::Gpu, || {
        headless_gpu_renderer("gpu_keeps_mirrored_single_sided_geometry_facing_outward")
    });
}

#[test]
fn cpu_keeps_facing_when_a_prepared_node_becomes_mirrored() {
    let renderer = Renderer::headless(SIZE, SIZE).expect("CPU renderer builds");
    assert_mirroring_a_prepared_node_keeps_it_visible(Backend::Cpu, renderer);
}

#[test]
fn gpu_keeps_facing_when_a_prepared_node_becomes_mirrored() {
    let Some(renderer) =
        headless_gpu_renderer("gpu_keeps_facing_when_a_prepared_node_becomes_mirrored")
    else {
        return;
    };
    assert_mirroring_a_prepared_node_keeps_it_visible(Backend::Gpu, renderer);
}

/// The GPU path re-uses the uploaded vertex buffers when only transforms
/// change. Mirroring a node after the first prepare must still reverse its
/// winding instead of leaving the single-sided plane back-face culled.
fn assert_mirroring_a_prepared_node_keeps_it_visible(backend: Backend, mut renderer: Renderer) {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = add_orthographic_camera(&mut scene);
    let parent = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("parent inserts");
    scene
        .directional_light(DirectionalLight::default())
        .add()
        .expect("light inserts");
    add_camera_facing_plane(&mut scene, &assets, parent, Vec3::new(0.6, 0.0, 0.0));
    let before = render(&mut renderer, &mut scene, &assets, camera);
    let before_bounds = covered_pixel_bounds(&before).expect("unmirrored plane renders");

    scene
        .set_transform(
            parent,
            Transform::IDENTITY.with_scale(Vec3::new(-1.0, 1.0, 1.0)),
        )
        .expect("parent mirrors");
    let after = render(&mut renderer, &mut scene, &assets, camera);
    let after_bounds = covered_pixel_bounds(&after).unwrap_or_else(|| {
        panic!("{backend:?}: a plane mirrored after the first prepare must stay visible")
    });

    let (before_pixels, after_pixels) = (covered_pixel_count(&before), covered_pixel_count(&after));
    assert!(
        before_pixels.abs_diff(after_pixels) * 50 <= before_pixels,
        "{backend:?}: mirroring must move the plane, not change its coverage: before={before_pixels}, after={after_pixels}"
    );
    let center = SIZE as f32 * 0.5;
    assert!(
        before_bounds.min_x > center && after_bounds.max_x < center,
        "{backend:?}: the plane must cross from the right half to the left half: before={before_bounds:?}, after={after_bounds:?}"
    );
}

#[test]
fn cpu_keeps_mirrored_instances_and_batched_nodes_visible() {
    assert_mirrored_instances_and_batched_nodes_stay_visible(Backend::Cpu, || {
        Some(Renderer::headless(SIZE, SIZE).expect("CPU renderer builds"))
    });
}

#[test]
fn gpu_keeps_mirrored_instances_and_batched_nodes_visible() {
    if headless_gpu_renderer("gpu_keeps_mirrored_instances_and_batched_nodes_visible").is_none() {
        return;
    }
    assert_mirrored_instances_and_batched_nodes_stay_visible(Backend::Gpu, || {
        headless_gpu_renderer("gpu_keeps_mirrored_instances_and_batched_nodes_visible")
    });
}

#[test]
fn cpu_views_the_scene_through_the_inverse_camera_world_matrix() {
    let renderer = Renderer::headless(SIZE, SIZE).expect("CPU renderer builds");
    assert_camera_under_sheared_parent_renders_where_it_projects(Backend::Cpu, renderer);
}

#[test]
fn gpu_views_the_scene_through_the_inverse_camera_world_matrix() {
    let Some(renderer) =
        headless_gpu_renderer("gpu_views_the_scene_through_the_inverse_camera_world_matrix")
    else {
        return;
    };
    assert_camera_under_sheared_parent_renders_where_it_projects(Backend::Gpu, renderer);
}

/// One instanced draw shares one triangle winding, and so does the template
/// of a batch of identical mesh nodes. A mirrored instance or a mirrored node
/// in such a group must still be drawn facing outward.
fn assert_mirrored_instances_and_batched_nodes_stay_visible(
    backend: Backend,
    mut renderer: impl FnMut() -> Option<Renderer>,
) {
    // Each copy covers about 26 x 20 pixels. The two rasterizers place pixel
    // centres slightly differently, so a copy can gain or lose an edge row or
    // column depending on where it sits; a missing copy loses a whole plane.
    let plane = || GeometryDesc::plane(0.52, 0.4);
    let covers = |measured: usize, planes: usize, single: usize| {
        measured.abs_diff(single * planes) * 100 <= single * planes * 15
    };
    let facing_camera = |at: Vec3| Transform::at(at).rotate_x_deg(90.0);
    let single = {
        let assets = Assets::new();
        let mut scene = Scene::new();
        let camera = add_orthographic_camera(&mut scene);
        let geometry = assets.create_geometry(plane());
        let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        scene
            .mesh(geometry, material)
            .transform(facing_camera(Vec3::ZERO))
            .add()
            .expect("reference plane inserts");
        let mut renderer = renderer().expect("renderer builds");
        covered_pixel_count(&render(&mut renderer, &mut scene, &assets, camera))
    };
    assert!(
        single > 400,
        "{backend:?}: reference plane must render, got {single}"
    );

    // An instance set whose second instance mirrors the geometry.
    let instanced = {
        let assets = Assets::new();
        let mut scene = Scene::new();
        let camera = add_orthographic_camera(&mut scene);
        let geometry = assets.create_geometry(plane());
        let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let set = scene
            .add_instance_set(scene.root(), geometry, material, Transform::IDENTITY)
            .expect("instance set inserts");
        scene
            .push_instance(set, facing_camera(Vec3::new(-1.0, 0.8, 0.0)))
            .expect("plain instance inserts");
        scene
            .push_instance(
                set,
                facing_camera(Vec3::new(1.0, 0.8, 0.0)).with_scale(Vec3::new(-1.0, 1.0, 1.0)),
            )
            .expect("mirrored instance inserts");
        let mut renderer = renderer().expect("renderer builds");
        covered_pixel_count(&render(&mut renderer, &mut scene, &assets, camera))
    };
    assert!(
        covers(instanced, 2, single),
        "{backend:?}: both the plain and the mirrored instance must render; one plane covers {single} pixels, the set covers {instanced}"
    );

    // Five identical mesh nodes are batched into one instanced draw on GPU
    // backends. The mirrored one cannot share the batch's winding.
    let batched = {
        let assets = Assets::new();
        let mut scene = Scene::new();
        let camera = add_orthographic_camera(&mut scene);
        let geometry = assets.create_geometry(plane());
        let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        for column in 0..5 {
            let at = Vec3::new(-1.4 + 0.7 * column as f32, -0.8, 0.0);
            let transform = if column == 2 {
                facing_camera(at).with_scale(Vec3::new(-1.0, 1.0, 1.0))
            } else {
                facing_camera(at)
            };
            scene
                .mesh(geometry, material)
                .transform(transform)
                .add()
                .expect("batched plane inserts");
        }
        let mut renderer = renderer().expect("renderer builds");
        covered_pixel_count(&render(&mut renderer, &mut scene, &assets, camera))
    };
    assert!(
        covers(batched, 5, single),
        "{backend:?}: all five planes must render, the mirrored one included; one plane covers {single} pixels, the batch covers {batched}"
    );
}

/// A camera under a non-uniformly scaled, rotated parent has a sheared world
/// matrix. Both backends must view the scene through its inverse, which is
/// also what `Scene::project_world_point` reports.
fn assert_camera_under_sheared_parent_renders_where_it_projects(
    backend: Backend,
    mut renderer: Renderer,
) {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let rig = scene
        .add_empty(
            scene.root(),
            Transform::at(Vec3::new(0.0, 0.5, 0.0))
                .rotate_y_deg(25.0)
                .with_scale(Vec3::new(2.5, 1.0, 1.0)),
        )
        .expect("camera rig inserts");
    let camera = scene
        .add_perspective_camera(
            rig,
            PerspectiveCamera::default().with_aspect(1.0),
            // Turned about Y, so the rig's X stretch shears the camera: its
            // X and Z axes end up about 45 degrees apart in the world.
            Transform::at(Vec3::new(0.3, 0.2, 5.0))
                .rotate_y_deg(40.0)
                .rotate_x_deg(-8.0),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    let camera_node = scene.camera_node(camera).expect("camera has a node");
    // A point chosen in the camera's own space, well off the optical axis, so
    // the ground truth is plain perspective arithmetic and any view matrix
    // other than the exact inverse puts the marker somewhere else.
    let in_view = Vec3::new(1.0, 0.5, -4.0);
    let marker_at = scene
        .world_matrix(camera_node)
        .expect("camera resolves")
        .transform_point3(in_view);
    let focal = (PerspectiveCamera::default().vertical_fov.radians() * 0.5)
        .tan()
        .recip();
    let half = SIZE as f32 * 0.5;
    let expected = (
        half + in_view.x * focal / -in_view.z * half,
        half - in_view.y * focal / -in_view.z * half,
    );
    let geometry = assets.create_geometry(GeometryDesc::sphere(0.25, 32, 24));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    scene
        .mesh(geometry, material)
        .transform(Transform::at(marker_at))
        .add()
        .expect("marker inserts");

    let projected = scene
        .project_world_point(camera, marker_at, SIZE, SIZE)
        .expect("camera resolves")
        .expect("the marker is in front of the camera");
    assert!(
        (projected.x - expected.0).abs() <= 0.05 && (projected.y - expected.1).abs() <= 0.05,
        "{backend:?}: project_world_point must invert the camera world matrix: expected {expected:?}, got ({}, {})",
        projected.x,
        projected.y
    );
    let frame = render(&mut renderer, &mut scene, &assets, camera);
    let bounds = covered_pixel_bounds(&frame)
        .unwrap_or_else(|| panic!("{backend:?}: the marker must be in view"));
    let centre = (
        (bounds.min_x + bounds.max_x) * 0.5,
        (bounds.min_y + bounds.max_y) * 0.5,
    );

    assert!(
        (centre.0 - expected.0).abs() <= 2.0 && (centre.1 - expected.1).abs() <= 2.0,
        "{backend:?}: the marker renders at {centre:?} but the inverse camera matrix projects it to {expected:?}"
    );
}

fn assert_bars_follow_the_world_matrix(backend: Backend, mut renderer: Renderer) {
    for hierarchy in BAR_HIERARCHIES {
        let assets = Assets::new();
        let mut scene = Scene::new();
        let camera = add_orthographic_camera(&mut scene);
        let bar = add_bar(&mut scene, &assets, hierarchy.chain);

        let world = scene.world_matrix(bar).expect("bar resolves");
        assert!(
            world.abs_diff_eq(ground_truth(hierarchy.chain), 1.0e-5),
            "{}: the scene must resolve the product of the local matrices",
            hierarchy.name
        );
        let expected = projected_bounds(world, BAR_HALF_SIZE);
        let frame = render(&mut renderer, &mut scene, &assets, camera);
        let measured = covered_pixel_bounds(&frame)
            .unwrap_or_else(|| panic!("{backend:?} {}: the bar must render", hierarchy.name));

        assert_bounds_near(backend, hierarchy.name, measured, expected);
    }
}

fn assert_reported_bar_aspect(backend: Backend, mut renderer: Renderer) {
    // The reported reproduction: a 1.0 x 0.2 x 0.2 bar turned a quarter turn
    // under a parent scaled (2, 1, 1) is 0.4 wide and 1.0 tall in the world.
    // Composing as TRS stretched it along its own length instead (0.2 x 2.0).
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = add_orthographic_camera(&mut scene);
    add_bar(&mut scene, &assets, BAR_HIERARCHIES[0].chain);

    let frame = render(&mut renderer, &mut scene, &assets, camera);
    let measured = covered_pixel_bounds(&frame).expect("the bar must render");
    let aspect = measured.width() / measured.height();

    assert!(
        (aspect - 0.4).abs() <= 0.03,
        "{backend:?}: bar width/height must be 0.400, got {aspect:.3} from {measured:?}"
    );
}

fn assert_mirrored_geometry_matches_its_twin(
    backend: Backend,
    mut renderer: impl FnMut() -> Option<Renderer>,
) {
    for (name, shape) in [("box", LitShape::Box), ("plane", LitShape::Plane)] {
        let mut mirrored_renderer = renderer().expect("renderer builds");
        let mut twin_renderer = renderer().expect("renderer builds");
        let mirrored = render_lit_shape(&mut mirrored_renderer, shape, true);
        let twin = render_lit_shape(&mut twin_renderer, shape, false);

        let twin_pixels = covered_pixel_count(&twin);
        assert!(
            twin_pixels > 1_000,
            "{backend:?} {name}: the unmirrored twin must cover the frame, got {twin_pixels} pixels"
        );
        assert!(
            distinct_covered_shades(&twin) >= shape.minimum_shades(),
            "{backend:?} {name}: the twin must be shaded, not a flat silhouette"
        );

        let silhouette_mismatch = silhouette_mismatch_count(&mirrored, &twin);
        assert!(
            silhouette_mismatch * 100 <= twin_pixels,
            "{backend:?} {name}: mirrored geometry must cover the same pixels as its twin; {silhouette_mismatch} of {twin_pixels} differ"
        );
        let mean_delta = mean_covered_channel_delta(&mirrored, &twin);
        assert!(
            mean_delta <= 3.0,
            "{backend:?} {name}: mirrored geometry must be lit like its twin (front faces outward, mirrored normals); mean channel delta {mean_delta:.2}"
        );
    }
}

#[derive(Debug, Clone, Copy)]
enum LitShape {
    Box,
    Plane,
}

impl LitShape {
    const fn minimum_shades(self) -> usize {
        match self {
            // Three visible faces under one directional light.
            Self::Box => 3,
            Self::Plane => 1,
        }
    }
}

/// Renders a lit single-sided shape either under a parent mirrored in X or as
/// the unmirrored twin placed where the mirror puts it.
fn render_lit_shape(renderer: &mut Renderer, shape: LitShape, mirrored: bool) -> Vec<u8> {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = add_orthographic_camera(&mut scene);
    // A key from the upper left and a weaker fill from the right, so every
    // visible face of the box receives a different amount of light.
    scene
        .directional_light(DirectionalLight::default().with_illuminance_lux(30_000.0))
        .transform(Transform::IDENTITY.rotate_y_deg(-35.0).rotate_x_deg(-30.0))
        .add()
        .expect("key light inserts");
    scene
        .directional_light(DirectionalLight::default().with_illuminance_lux(9_000.0))
        .transform(Transform::IDENTITY.rotate_y_deg(65.0).rotate_x_deg(-10.0))
        .add()
        .expect("fill light inserts");

    let translation = Vec3::new(0.7, 0.2, 0.0);
    let rotation = match shape {
        LitShape::Box => {
            Transform::IDENTITY
                .rotate_y_deg(35.0)
                .rotate_x_deg(25.0)
                .rotation
        }
        // The plane's +Y normal turned to face the camera, then leaned so the
        // mirror changes which way it leans.
        LitShape::Plane => {
            Transform::IDENTITY
                .rotate_y_deg(20.0)
                .rotate_x_deg(90.0)
                .rotation
        }
    };
    let (parent, local) = if mirrored {
        let parent = scene
            .add_empty(
                scene.root(),
                Transform::IDENTITY.with_scale(Vec3::new(-1.0, 1.0, 1.0)),
            )
            .expect("mirrored parent inserts");
        (
            parent,
            Transform {
                translation,
                rotation,
                scale: Vec3::ONE,
            },
        )
    } else {
        // Reflecting a rotation in X keeps its X axis component and negates
        // the other two; both shapes are symmetric in X, so no scale remains.
        (
            scene.root(),
            Transform {
                translation: Vec3::new(-translation.x, translation.y, translation.z),
                rotation: Quat::from_xyzw(rotation.x, -rotation.y, -rotation.z, rotation.w),
                scale: Vec3::ONE,
            },
        )
    };
    let geometry = assets.create_geometry(match shape {
        LitShape::Box => GeometryDesc::box_xyz(1.0, 0.7, 0.5),
        LitShape::Plane => GeometryDesc::plane(1.2, 0.8),
    });
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_linear_rgb(0.8, 0.45, 0.2),
        0.0,
        0.9,
    ));
    scene
        .mesh(geometry, material)
        .parent(parent)
        .transform(local)
        .add()
        .expect("lit mesh inserts");
    render(renderer, &mut scene, &assets, camera)
}

fn add_bar(scene: &mut Scene, assets: &Assets, chain: &[Transform]) -> NodeKey {
    let (mesh_local, ancestors) = chain.split_last().expect("hierarchy has a mesh node");
    let mut parent = scene.root();
    for local in ancestors {
        parent = scene
            .add_empty(parent, *local)
            .expect("ancestor node inserts");
    }
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(
        BAR_HALF_SIZE.x * 2.0,
        BAR_HALF_SIZE.y * 2.0,
        BAR_HALF_SIZE.z * 2.0,
    ));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    scene
        .mesh(geometry, material)
        .parent(parent)
        .transform(*mesh_local)
        .add()
        .expect("bar mesh inserts")
}

/// A lit, single-sided plane. Backend-shaded materials are the ones whose
/// uploaded vertex buffers the GPU path re-uses across transform-only prepares.
fn add_camera_facing_plane(scene: &mut Scene, assets: &Assets, parent: NodeKey, at: Vec3) {
    let geometry = assets.create_geometry(GeometryDesc::plane(0.8, 0.6));
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8));
    scene
        .mesh(geometry, material)
        .parent(parent)
        .transform(Transform::at(at).rotate_x_deg(90.0))
        .add()
        .expect("plane inserts");
}

fn add_orthographic_camera(scene: &mut Scene) -> CameraKey {
    let camera = scene
        .add_orthographic_camera(
            scene.root(),
            OrthographicCamera {
                left: -HALF_EXTENT,
                right: HALF_EXTENT,
                bottom: -HALF_EXTENT,
                top: HALF_EXTENT,
                near: 0.1,
                far: 20.0,
            },
            Transform::at(Vec3::new(0.0, 0.0, 10.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    camera
}

fn render(
    renderer: &mut Renderer,
    scene: &mut Scene,
    assets: &Assets,
    camera: CameraKey,
) -> Vec<u8> {
    renderer.set_background_color(Color::from_linear_rgb(0.0, 0.02, 0.2));
    renderer
        .prepare_with_assets(scene, assets)
        .expect("scene prepares");
    renderer.render(scene, camera).expect("scene renders");
    renderer.frame_rgba8().to_vec()
}

fn headless_gpu_renderer(test_name: &str) -> Option<Renderer> {
    support::parity::configure_lavapipe_adapter();
    match Renderer::headless_gpu(SIZE, SIZE) {
        Ok(renderer) => Some(renderer),
        Err(error) => {
            eprintln!("skipping {test_name}: no headless GPU adapter is available ({error:?})");
            None
        }
    }
}

fn matrix_of(local: Transform) -> Mat4 {
    Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.translation)
}

fn ground_truth(chain: &[Transform]) -> Mat4 {
    chain
        .iter()
        .fold(Mat4::IDENTITY, |world, local| world * matrix_of(*local))
}

/// Pixel bounds of a box of `half_size` carried through `world` and the
/// orthographic test camera.
fn projected_bounds(world: Mat4, half_size: Vec3) -> PixelBounds {
    let mut bounds = PixelBounds {
        min_x: f32::INFINITY,
        max_x: f32::NEG_INFINITY,
        min_y: f32::INFINITY,
        max_y: f32::NEG_INFINITY,
    };
    for corner in 0..8 {
        let local = Vec3::new(
            if corner & 1 == 0 {
                -half_size.x
            } else {
                half_size.x
            },
            if corner & 2 == 0 {
                -half_size.y
            } else {
                half_size.y
            },
            if corner & 4 == 0 {
                -half_size.z
            } else {
                half_size.z
            },
        );
        let point = world.transform_point3(local);
        let x = (point.x + HALF_EXTENT) / (HALF_EXTENT * 2.0) * SIZE as f32;
        let y = (HALF_EXTENT - point.y) / (HALF_EXTENT * 2.0) * SIZE as f32;
        bounds.min_x = bounds.min_x.min(x);
        bounds.max_x = bounds.max_x.max(x);
        bounds.min_y = bounds.min_y.min(y);
        bounds.max_y = bounds.max_y.max(y);
    }
    bounds
}

/// The clear colour, read from a corner no test geometry reaches.
fn background(frame: &[u8]) -> [u8; 3] {
    [frame[0], frame[1], frame[2]]
}

/// Whether a pixel shows geometry rather than the clear colour. Measuring
/// against the background keeps unlit faces inside the silhouette.
fn is_covered(pixel: &[u8], background: [u8; 3]) -> bool {
    (0..3).any(|channel| pixel[channel].abs_diff(background[channel]) > 12)
}

fn covered_pixel_count(frame: &[u8]) -> usize {
    let background = background(frame);
    frame
        .chunks_exact(4)
        .filter(|pixel| is_covered(pixel, background))
        .count()
}

/// Bounds of the covered pixels, measured on pixel edges.
fn covered_pixel_bounds(frame: &[u8]) -> Option<PixelBounds> {
    let background = background(frame);
    let mut bounds: Option<PixelBounds> = None;
    for (index, pixel) in frame.chunks_exact(4).enumerate() {
        if !is_covered(pixel, background) {
            continue;
        }
        let x = (index as u32 % SIZE) as f32;
        let y = (index as u32 / SIZE) as f32;
        let current = bounds.get_or_insert(PixelBounds {
            min_x: x,
            max_x: x + 1.0,
            min_y: y,
            max_y: y + 1.0,
        });
        current.min_x = current.min_x.min(x);
        current.max_x = current.max_x.max(x + 1.0);
        current.min_y = current.min_y.min(y);
        current.max_y = current.max_y.max(y + 1.0);
    }
    bounds
}

fn assert_bounds_near(backend: Backend, name: &str, measured: PixelBounds, expected: PixelBounds) {
    let near = |measured: f32, expected: f32| (measured - expected).abs() <= BOUNDS_TOLERANCE_PX;
    assert!(
        near(measured.min_x, expected.min_x)
            && near(measured.max_x, expected.max_x)
            && near(measured.min_y, expected.min_y)
            && near(measured.max_y, expected.max_y),
        "{backend:?} {name}: rendered bounds {measured:?} must match the world-matrix bounds {expected:?}"
    );
}

fn silhouette_mismatch_count(left: &[u8], right: &[u8]) -> usize {
    let (left_background, right_background) = (background(left), background(right));
    left.chunks_exact(4)
        .zip(right.chunks_exact(4))
        .filter(|(left, right)| {
            is_covered(left, left_background) != is_covered(right, right_background)
        })
        .count()
}

/// Mean absolute RGB difference over the pixels covered in both frames.
fn mean_covered_channel_delta(left: &[u8], right: &[u8]) -> f32 {
    let (left_background, right_background) = (background(left), background(right));
    let mut total = 0_u64;
    let mut channels = 0_u64;
    for (left, right) in left.chunks_exact(4).zip(right.chunks_exact(4)) {
        if !(is_covered(left, left_background) && is_covered(right, right_background)) {
            continue;
        }
        for channel in 0..3 {
            total += u64::from(left[channel].abs_diff(right[channel]));
            channels += 1;
        }
    }
    assert!(channels > 0, "the frames must share covered pixels");
    total as f32 / channels as f32
}

fn distinct_covered_shades(frame: &[u8]) -> usize {
    let background = background(frame);
    frame
        .chunks_exact(4)
        .filter(|pixel| is_covered(pixel, background))
        .map(|pixel| [pixel[0] / 8, pixel[1] / 8, pixel[2] / 8])
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}
