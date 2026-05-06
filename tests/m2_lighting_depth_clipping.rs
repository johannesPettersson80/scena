#![cfg(not(target_arch = "wasm32"))]

use scena::{
    Angle, Color, DirectionalLight, Light, NodeKind, PointLight, Scene, SpotLight, Transform, Vec3,
};

#[test]
fn scene_light_components_are_typed_and_node_owned() {
    let mut scene = Scene::new();
    let parent = scene
        .add_empty(
            scene.root(),
            Transform {
                translation: Vec3::new(1.0, 2.0, 3.0),
                ..Transform::default()
            },
        )
        .expect("light parent inserts");
    let directional_transform = Transform {
        translation: Vec3::new(0.0, 4.0, 0.0),
        ..Transform::default()
    };
    let point_transform = Transform {
        translation: Vec3::new(2.0, 3.0, 4.0),
        ..Transform::default()
    };
    let spot_transform = Transform {
        translation: Vec3::new(-2.0, 5.0, 1.0),
        ..Transform::default()
    };

    let directional_node = scene
        .directional_light(
            DirectionalLight::default()
                .with_color(Color::from_linear_rgb(1.0, 0.96, 0.9))
                .with_illuminance_lux(20_000.0),
        )
        .parent(parent)
        .transform(directional_transform)
        .add()
        .expect("directional light inserts");
    let point_node = scene
        .point_light(
            PointLight::default()
                .with_color(Color::from_linear_rgb(0.6, 0.8, 1.0))
                .with_intensity_candela(600.0)
                .with_range(12.0),
        )
        .transform(point_transform)
        .add()
        .expect("point light inserts");
    let spot_node = scene
        .spot_light(
            SpotLight::default()
                .with_intensity_candela(800.0)
                .with_range(20.0)
                .with_inner_cone_angle(Angle::from_degrees(12.0))
                .with_outer_cone_angle(Angle::from_degrees(30.0)),
        )
        .transform(spot_transform)
        .add()
        .expect("spot light inserts");

    let directional_key = match scene.node(directional_node).expect("node exists").kind() {
        NodeKind::Light(light) => *light,
        kind => panic!("expected directional light node, got {kind:?}"),
    };
    let point_key = match scene.node(point_node).expect("node exists").kind() {
        NodeKind::Light(light) => *light,
        kind => panic!("expected point light node, got {kind:?}"),
    };
    let spot_key = match scene.node(spot_node).expect("node exists").kind() {
        NodeKind::Light(light) => *light,
        kind => panic!("expected spot light node, got {kind:?}"),
    };

    assert_eq!(
        scene.node(directional_node).expect("node exists").parent(),
        Some(parent)
    );
    assert_eq!(
        scene
            .node(directional_node)
            .expect("node exists")
            .transform(),
        directional_transform
    );
    assert_eq!(
        scene.node(point_node).expect("node exists").transform(),
        point_transform
    );
    assert_eq!(
        scene.node(spot_node).expect("node exists").transform(),
        spot_transform
    );
    assert!(matches!(
        scene.light(directional_key),
        Some(Light::Directional(light))
            if light.illuminance_lux() == 20_000.0
                && light.color() == Color::from_linear_rgb(1.0, 0.96, 0.9)
    ));
    assert!(matches!(
        scene.light(point_key),
        Some(Light::Point(light)) if light.intensity_candela() == 600.0 && light.range() == Some(12.0)
    ));
    assert!(matches!(
        scene.light(spot_key),
        Some(Light::Spot(light))
            if light.intensity_candela() == 800.0
                && light.range() == Some(20.0)
                && light.inner_cone_angle() == Angle::from_degrees(12.0)
                && light.outer_cone_angle() == Angle::from_degrees(30.0)
    ));
}
