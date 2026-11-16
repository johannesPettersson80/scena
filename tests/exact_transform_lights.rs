//! Rendered proof that lights are placed by the exact world matrix of their
//! node, on the CPU rasterizer and, when a headless adapter is available, on
//! the GPU path.
//!
//! A directional or spot light under a rotated, non-uniformly scaled parent
//! must light the scene exactly like a top-level twin whose direction and
//! position were computed with plain matrix products. An area light has no
//! such twin, because a sheared rectangle is a parallelogram, so its footprint
//! on a floor is measured against the matrix column instead.
#![cfg(not(target_arch = "wasm32"))]

#[allow(dead_code)]
mod support;

use scena::{
    AreaLight, AreaLightShape, Assets, CameraKey, Color, DirectionalLight, GeometryDesc, Mat4,
    MaterialDesc, OrthographicCamera, Renderer, Scene, SpotLight, Transform, Vec3,
};

const SIZE: u32 = 160;
/// World units visible on each side of the view centre.
const HALF_EXTENT: f32 = 2.0;
/// Dim enough that the floor under the emitter does not saturate.
const AREA_FLUX_LUMENS: f32 = 40.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Cpu,
    Gpu,
}

#[derive(Debug, Clone, Copy)]
enum LightKind {
    Directional,
    Spot,
}

/// A rotated, non-uniformly scaled parent with a rotated light below it: the
/// light's world matrix is sheared and aims down at the floor.
fn sheared_chain() -> [Transform; 2] {
    [
        Transform::at(Vec3::new(0.3, 1.5, 0.4))
            .rotate_y_deg(25.0)
            .with_scale(Vec3::new(2.0, 1.0, 0.6)),
        Transform::at(Vec3::new(0.0, 0.1, 0.0))
            .rotate_x_deg(-45.0)
            .rotate_z_deg(10.0),
    ]
}

#[test]
fn cpu_directional_light_under_a_sheared_parent_matches_its_matrix_placed_twin() {
    assert_light_matches_its_twin(Backend::Cpu, LightKind::Directional);
}

#[test]
fn gpu_directional_light_under_a_sheared_parent_matches_its_matrix_placed_twin() {
    if headless_gpu_renderer("gpu_directional_light_under_a_sheared_parent").is_none() {
        return;
    }
    assert_light_matches_its_twin(Backend::Gpu, LightKind::Directional);
}

#[test]
fn cpu_spot_light_under_a_sheared_parent_matches_its_matrix_placed_twin() {
    assert_light_matches_its_twin(Backend::Cpu, LightKind::Spot);
}

#[test]
fn gpu_spot_light_under_a_sheared_parent_matches_its_matrix_placed_twin() {
    if headless_gpu_renderer("gpu_spot_light_under_a_sheared_parent").is_none() {
        return;
    }
    assert_light_matches_its_twin(Backend::Gpu, LightKind::Spot);
}

#[test]
fn cpu_area_light_footprint_follows_the_sheared_matrix_axes() {
    assert_area_light_footprint_follows_the_matrix(Backend::Cpu);
}

#[test]
fn gpu_area_light_footprint_follows_the_sheared_matrix_axes() {
    if headless_gpu_renderer("gpu_area_light_footprint_follows_the_sheared_matrix_axes").is_none() {
        return;
    }
    assert_area_light_footprint_follows_the_matrix(Backend::Gpu);
}

fn assert_light_matches_its_twin(backend: Backend, kind: LightKind) {
    let chain = sheared_chain();
    let world = ground_truth(&chain);
    let position = world.transform_point3(Vec3::ZERO);
    let direction = world.transform_vector3(Vec3::NEG_Z).normalize();
    assert!(
        direction.y < -0.8,
        "the fixture must aim the light at the floor: {direction:?}"
    );
    // What composing translation, rotation, and scale component by component
    // would have aimed the light along. It must differ visibly, or the twin
    // comparison could not tell the two apart.
    let approximate = Transform::compose(chain[0], chain[1]).rotation * Vec3::NEG_Z;
    assert!(
        approximate.angle_between(direction).to_degrees() > 10.0,
        "the fixture must shear the light's forward axis"
    );

    let parented = render_lit_scene(backend, kind, |scene| {
        let parent = scene
            .add_empty(scene.root(), chain[0])
            .expect("parent inserts");
        (parent, chain[1])
    });
    let twin = render_lit_scene(backend, kind, |scene| {
        (
            scene.root(),
            Transform::at(position).looking_at(position + direction, Vec3::Y),
        )
    });
    let mis_aimed = render_lit_scene(backend, kind, |scene| {
        (
            scene.root(),
            Transform::at(position).looking_at(position + approximate, Vec3::Y),
        )
    });

    let lit = lit_pixel_count(&twin);
    assert!(
        lit > 400,
        "{backend:?} {kind:?}: the twin must light the scene; lit pixels={lit}"
    );
    let difference = mean_channel_difference(&parented, &twin);
    let control = mean_channel_difference(&mis_aimed, &twin);
    assert!(
        control > 1.0,
        "{backend:?} {kind:?}: a mis-aimed light must be visible to this comparison; control={control}"
    );
    assert!(
        difference < 0.05 * control && difference < 0.5,
        "{backend:?} {kind:?}: the parented light must render like its matrix-placed twin; \
         mean difference={difference}, mis-aimed control={control}"
    );
}

/// Renders a floor and a box lit by one light that `place` parents and
/// positions, seen from an oblique orthographic camera.
fn render_lit_scene(
    backend: Backend,
    kind: LightKind,
    place: impl FnOnce(&mut Scene) -> (scena::NodeKey, Transform),
) -> Vec<u8> {
    let mut renderer = renderer_for(backend);
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = add_orthographic_camera(
        &mut scene,
        Transform::at(Vec3::new(4.0, 5.0, 4.0)).looking_at(Vec3::ZERO, Vec3::Y),
    );
    add_floor(&mut scene, &assets);
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.9, 0.5, 0.7));
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.9));
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-0.2, 0.25, -0.3)))
        .add()
        .expect("box inserts");
    let (parent, local) = place(&mut scene);
    match kind {
        LightKind::Directional => scene
            .directional_light(DirectionalLight::default().with_illuminance_lux(30_000.0))
            .parent(parent)
            .transform(local)
            .add(),
        LightKind::Spot => scene
            .spot_light(
                SpotLight::default()
                    .with_intensity_candela(40_000.0)
                    .with_inner_cone_angle(scena::Angle::from_degrees(14.0))
                    .with_outer_cone_angle(scena::Angle::from_degrees(22.0)),
            )
            .parent(parent)
            .transform(local)
            .add(),
    }
    .expect("light inserts");
    render(&mut renderer, &mut scene, &assets, camera)
}

fn assert_area_light_footprint_follows_the_matrix(backend: Backend) {
    // The parent stretches X; the light lies flat above the floor and is
    // turned 45 degrees in that plane, so its long axis is sheared.
    let chain = [
        Transform::at(Vec3::new(0.0, 0.45, 0.0))
            .rotate_y_deg(20.0)
            .with_scale(Vec3::new(3.0, 1.0, 1.0)),
        Transform::IDENTITY.rotate_y_deg(45.0).rotate_x_deg(-90.0),
    ];
    let world = ground_truth(&chain);
    let long_axis = world.transform_vector3(Vec3::X).normalize();
    let short_axis = world.transform_vector3(Vec3::Y).normalize();
    assert!(
        long_axis.y.abs() < 1.0e-5 && short_axis.y.abs() < 1.0e-5,
        "the fixture must keep the emitter parallel to the floor"
    );
    assert!(
        long_axis.dot(short_axis).abs() > 0.5,
        "the fixture must shear the emitter: {long_axis:?} {short_axis:?}"
    );

    let mut renderer = renderer_for(backend);
    let assets = Assets::new();
    let mut scene = Scene::new();
    // Straight down, with -Z up the image: pixel x follows world X and pixel
    // y follows world Z.
    let camera = add_orthographic_camera(
        &mut scene,
        Transform::at(Vec3::new(0.0, 10.0, 0.0)).looking_at(Vec3::ZERO, Vec3::NEG_Z),
    );
    add_floor(&mut scene, &assets);
    let parent = scene
        .add_empty(scene.root(), chain[0])
        .expect("parent inserts");
    scene
        .area_light(
            AreaLight::default()
                .with_luminous_flux_lumens(AREA_FLUX_LUMENS)
                .with_shape(AreaLightShape::rect(2.0, 0.12)),
        )
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("area light inserts");
    let frame = render(&mut renderer, &mut scene, &assets, camera);

    let footprint = brightness_moments(&frame).unwrap_or_else(|| {
        panic!(
            "{backend:?}: the area light must light the floor; luminance range={:?}",
            luminance_range(&frame)
        );
    });
    assert!(
        footprint.elongation > 1.25,
        "{backend:?}: the footprint must be long enough to have an axis: {footprint:?}"
    );
    let expected = long_axis.z.atan2(long_axis.x).to_degrees();
    let error = axis_angle_difference(footprint.axis_degrees, expected);
    assert!(
        error < 4.0,
        "{backend:?}: the footprint runs along {:.1} degrees but the emitter's long axis, \
         the matrix X column, runs along {expected:.1} degrees (off by {error:.1})",
        footprint.axis_degrees
    );
}

#[derive(Debug, Clone, Copy)]
struct Footprint {
    /// Direction of the long axis in the image, in degrees from pixel +x.
    axis_degrees: f32,
    /// Ratio of the spread along the long axis to the spread across it.
    elongation: f32,
}

fn luminance(pixel: &[u8]) -> f64 {
    f64::from(pixel[0]) + f64::from(pixel[1]) + f64::from(pixel[2])
}

/// Darkest and brightest summed channel value in the frame.
fn luminance_range(frame: &[u8]) -> (f64, f64) {
    frame
        .chunks_exact(4)
        .map(luminance)
        .fold((f64::INFINITY, 0.0), |(floor, peak), value| {
            (floor.min(value), peak.max(value))
        })
}

/// Principal axis of the brightness above the darkest frame level.
fn brightness_moments(frame: &[u8]) -> Option<Footprint> {
    let (floor, peak) = luminance_range(frame);
    if peak - floor < 30.0 {
        return None;
    }
    let cutoff = floor + (peak - floor) * 0.35;
    let mut total = 0.0;
    let mut sum = [0.0_f64; 2];
    let weighted = frame
        .chunks_exact(4)
        .enumerate()
        .filter_map(|(index, pixel)| {
            let weight = luminance(pixel) - cutoff;
            (weight > 0.0).then(|| {
                let x = (index as u32 % SIZE) as f64;
                let y = (index as u32 / SIZE) as f64;
                (x, y, weight)
            })
        })
        .collect::<Vec<_>>();
    for (x, y, weight) in &weighted {
        total += weight;
        sum[0] += weight * x;
        sum[1] += weight * y;
    }
    let mean = [sum[0] / total, sum[1] / total];
    let (mut xx, mut yy, mut xy) = (0.0, 0.0, 0.0);
    for (x, y, weight) in &weighted {
        let (dx, dy) = (x - mean[0], y - mean[1]);
        xx += weight * dx * dx;
        yy += weight * dy * dy;
        xy += weight * dx * dy;
    }
    let angle = 0.5 * (2.0 * xy).atan2(xx - yy);
    let spread = ((xx - yy) * (xx - yy) + 4.0 * xy * xy).sqrt();
    let major = (xx + yy + spread) * 0.5;
    let minor = ((xx + yy - spread) * 0.5).max(f64::EPSILON);
    Some(Footprint {
        axis_degrees: angle.to_degrees() as f32,
        elongation: (major / minor).sqrt() as f32,
    })
}

/// Smallest angle between two undirected axes, in degrees.
fn axis_angle_difference(left: f32, right: f32) -> f32 {
    let difference = (left - right).rem_euclid(180.0);
    difference.min(180.0 - difference)
}

fn lit_pixel_count(frame: &[u8]) -> usize {
    frame
        .chunks_exact(4)
        .filter(|pixel| u32::from(pixel[0]) + u32::from(pixel[1]) + u32::from(pixel[2]) > 90)
        .count()
}

fn mean_channel_difference(left: &[u8], right: &[u8]) -> f64 {
    assert_eq!(left.len(), right.len());
    let total: u64 = left
        .iter()
        .zip(right)
        .map(|(left, right)| u64::from(left.abs_diff(*right)))
        .sum();
    total as f64 / left.len() as f64
}

fn add_floor(scene: &mut Scene, assets: &Assets) {
    let geometry = assets.create_geometry(GeometryDesc::plane(8.0, 8.0));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::WHITE,
        0.0,
        0.95,
    ));
    scene.mesh(geometry, material).add().expect("floor inserts");
}

fn add_orthographic_camera(scene: &mut Scene, transform: Transform) -> CameraKey {
    let camera = scene
        .add_orthographic_camera(
            scene.root(),
            OrthographicCamera {
                left: -HALF_EXTENT,
                right: HALF_EXTENT,
                bottom: -HALF_EXTENT,
                top: HALF_EXTENT,
                near: 0.1,
                far: 30.0,
            },
            transform,
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    camera
}

fn renderer_for(backend: Backend) -> Renderer {
    match backend {
        Backend::Cpu => Renderer::headless(SIZE, SIZE).expect("CPU renderer builds"),
        Backend::Gpu => headless_gpu_renderer("exact_transform_lights")
            .expect("the GPU twin checked for an adapter before rendering"),
    }
}

fn render(
    renderer: &mut Renderer,
    scene: &mut Scene,
    assets: &Assets,
    camera: CameraKey,
) -> Vec<u8> {
    renderer.set_background_color(Color::BLACK);
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
