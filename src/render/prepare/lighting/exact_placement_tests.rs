//! Lights take their direction, position, and area axes from the exact world
//! matrix of their node. Every expectation here is a plain matrix product.

use super::math::{normalize_or, rotate_vec3};
use super::*;
use crate::scene::{
    AreaLight, AreaLightShape, DirectionalLight, Mat4, NodeKey, PointLight, SpotLight, Transform,
};

/// A rotated, non-uniformly scaled parent with a rotated light below it, so
/// the light's world matrix is sheared.
fn sheared_chain() -> [Transform; 2] {
    [
        Transform::at(Vec3::new(0.4, 1.2, -0.3))
            .rotate_y_deg(25.0)
            .with_scale(Vec3::new(3.0, 1.0, 0.6)),
        Transform::at(Vec3::new(0.3, 0.5, 0.2))
            .rotate_z_deg(40.0)
            .rotate_x_deg(-20.0),
    ]
}

fn matrix_of(local: Transform) -> Mat4 {
    Mat4::from_scale_rotation_translation(local.scale, local.rotation, local.translation)
}

fn world_matrix(chain: &[Transform]) -> Mat4 {
    chain
        .iter()
        .fold(Mat4::IDENTITY, |world, local| world * matrix_of(*local))
}

fn add_parent(scene: &mut Scene, parent: Transform) -> NodeKey {
    scene
        .add_empty(scene.root(), parent)
        .expect("light parent inserts")
}

fn uniform_of(scene: &Scene, origin_shift: Vec3) -> PreparedGpuLightUniform {
    collect_gpu_light_uniform(
        scene,
        origin_shift,
        &PreparedEnvironmentLighting::default(),
        false,
    )
}

fn xyz(lane: [f32; 4]) -> Vec3 {
    Vec3::new(lane[0], lane[1], lane[2])
}

fn assert_close(actual: Vec3, expected: Vec3, what: &str) {
    assert!(
        (actual - expected).length() <= 2.0e-5 * (1.0 + expected.length()),
        "{what}: expected {expected:?}, got {actual:?}"
    );
}

fn area_light(shape: AreaLightShape) -> AreaLight {
    AreaLight::default()
        .with_luminous_flux_lumens(400.0)
        .with_shape(shape)
}

#[test]
fn rect_and_disc_area_light_axes_follow_the_sheared_world_matrix() {
    let chain = sheared_chain();
    let world = world_matrix(&chain);
    let right = world.transform_vector3(Vec3::X).normalize();
    let up = world.transform_vector3(Vec3::Y).normalize();
    assert!(
        right.dot(up).abs() > 0.2,
        "the fixture must shear the light plane, or it proves nothing"
    );

    let mut scene = Scene::new();
    let parent = add_parent(&mut scene, chain[0]);
    scene
        .area_light(area_light(AreaLightShape::rect(1.6, 0.4)))
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("rect light inserts");
    scene
        .area_light(area_light(AreaLightShape::disc(0.3)))
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("disc light inserts");

    let uniform = uniform_of(&scene, Vec3::ZERO);

    assert_close(
        xyz(uniform.area_light_axis_x_shape[0]),
        right * 0.8,
        "rect half-width axis",
    );
    assert_close(
        xyz(uniform.area_light_axis_y_range[0]),
        up * 0.2,
        "rect half-height axis",
    );
    assert_close(
        xyz(uniform.area_light_axis_x_shape[1]),
        right * 0.3,
        "disc first radius axis",
    );
    assert_close(
        xyz(uniform.area_light_axis_y_range[1]),
        up * 0.3,
        "disc second radius axis",
    );
}

#[test]
fn light_directions_follow_the_sheared_world_matrix() {
    let chain = sheared_chain();
    let forward = world_matrix(&chain)
        .transform_vector3(Vec3::NEG_Z)
        .normalize();

    let mut scene = Scene::new();
    let parent = add_parent(&mut scene, chain[0]);
    scene
        .directional_light(DirectionalLight::default())
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("directional light inserts");
    scene
        .spot_light(SpotLight::default())
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("spot light inserts");

    let uniform = uniform_of(&scene, Vec3::ZERO);

    assert_close(
        xyz(uniform.directional_light_direction_intensity[0]),
        forward,
        "directional light direction",
    );
    assert_close(
        xyz(uniform.spot_light_direction_cones[0]),
        forward,
        "spot light direction",
    );
}

#[test]
fn light_positions_follow_the_sheared_world_matrix_and_the_origin_shift() {
    let chain = sheared_chain();
    let origin_shift = Vec3::new(10.0, -4.0, 2.5);
    let position = world_matrix(&chain).transform_point3(Vec3::ZERO) - origin_shift;

    let mut scene = Scene::new();
    let parent = add_parent(&mut scene, chain[0]);
    scene
        .point_light(PointLight::default())
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("point light inserts");
    scene
        .spot_light(SpotLight::default())
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("spot light inserts");
    scene
        .area_light(area_light(AreaLightShape::rect(1.0, 1.0)))
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("area light inserts");

    let uniform = uniform_of(&scene, origin_shift);

    assert_close(
        xyz(uniform.point_light_position_intensity[0]),
        position,
        "point light position",
    );
    assert_close(
        xyz(uniform.spot_light_position_intensity[0]),
        position,
        "spot light position",
    );
    assert_close(
        xyz(uniform.area_light_position_flux[0]),
        position,
        "area light position",
    );
}

/// A sphere has no orientation, and a node's scale never resizes a light, so
/// shear must leave a sphere light a sphere of its authored radius.
#[test]
fn sphere_area_light_keeps_an_orthonormal_frame_under_shear() {
    let chain = sheared_chain();
    let mut scene = Scene::new();
    let parent = add_parent(&mut scene, chain[0]);
    scene
        .area_light(area_light(AreaLightShape::sphere(0.25)))
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("sphere light inserts");

    let uniform = uniform_of(&scene, Vec3::ZERO);
    let axis_x = xyz(uniform.area_light_axis_x_shape[0]);
    let axis_y = xyz(uniform.area_light_axis_y_range[0]);

    assert!((axis_x.length() - 0.25).abs() <= 1.0e-6, "{axis_x:?}");
    assert!((axis_y.length() - 0.25).abs() <= 1.0e-6, "{axis_y:?}");
    assert!(axis_x.dot(axis_y).abs() <= 1.0e-6, "{axis_x:?} {axis_y:?}");
}

/// A shear-free chain that mirrors the light through its own forward axis.
/// The same chain with any rotation below the mirror is sheared and already
/// follows the matrix, so the unrotated case must agree with it.
#[test]
fn a_light_mirrored_through_its_forward_axis_points_where_its_matrix_points() {
    let chain = [
        Transform::IDENTITY
            .rotate_y_deg(30.0)
            .with_scale(Vec3::new(1.0, 1.0, -1.0)),
        Transform::at(Vec3::new(0.0, 0.5, 0.0)),
    ];
    let forward = world_matrix(&chain)
        .transform_vector3(Vec3::NEG_Z)
        .normalize();

    let mut scene = Scene::new();
    let parent = add_parent(&mut scene, chain[0]);
    scene
        .directional_light(DirectionalLight::default())
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("directional light inserts");
    scene
        .spot_light(SpotLight::default())
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("spot light inserts");

    let uniform = uniform_of(&scene, Vec3::ZERO);

    assert_close(
        xyz(uniform.directional_light_direction_intensity[0]),
        forward,
        "mirrored directional light direction",
    );
    assert_close(
        xyz(uniform.spot_light_direction_cones[0]),
        forward,
        "mirrored spot light direction",
    );
}

/// Hierarchies of translation, rotation, and positive scale keep the
/// rotation-only arithmetic bit for bit.
#[test]
fn shear_free_lights_keep_their_rotation_only_arithmetic() {
    let chain = [
        Transform::at(Vec3::new(0.4, 1.2, -0.3))
            .rotate_z_deg(25.0)
            .scale_by(1.7),
        Transform::at(Vec3::new(0.3, 0.5, 0.2))
            .rotate_x_deg(-50.0)
            .rotate_y_deg(35.0)
            .with_scale(Vec3::new(1.5, 0.5, 2.0)),
    ];
    let mut scene = Scene::new();
    let parent = add_parent(&mut scene, chain[0]);
    let light_node = scene
        .directional_light(DirectionalLight::default())
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("directional light inserts");
    scene
        .spot_light(SpotLight::default())
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("spot light inserts");
    scene
        .area_light(area_light(AreaLightShape::rect(1.6, 0.4)))
        .parent(parent)
        .transform(chain[1])
        .add()
        .expect("rect light inserts");
    let rotation = scene
        .world_transform(light_node)
        .expect("light node resolves")
        .rotation;
    let forward = normalize_or(
        rotate_vec3(rotation, Vec3::new(0.0, 0.0, -1.0)),
        Vec3::new(0.0, 0.0, -1.0),
    );

    let uniform = uniform_of(&scene, Vec3::ZERO);

    assert_eq!(
        xyz(uniform.directional_light_direction_intensity[0]),
        forward
    );
    assert_eq!(xyz(uniform.spot_light_direction_cones[0]), forward);
    assert_eq!(
        xyz(uniform.area_light_axis_x_shape[0]),
        rotate_vec3(rotation, Vec3::X) * 0.8
    );
    assert_eq!(
        xyz(uniform.area_light_axis_y_range[0]),
        rotate_vec3(rotation, Vec3::Y) * 0.2
    );
}
