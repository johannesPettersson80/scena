use super::*;
use crate::scene::{
    Angle, AreaLight, AreaLightShape, DirectionalLight, PointLight, SpotLight, Transform,
};

#[test]
fn gpu_light_uniform_consumes_prepared_environment_lighting() {
    let scene = Scene::new();
    let environment = PreparedEnvironmentLighting::default();

    let uniform = collect_gpu_light_uniform(&scene, Vec3::ZERO, &environment, false);

    assert_eq!(uniform.environment_diffuse_intensity, [0.0; 4]);
    assert_eq!(uniform.environment_specular_intensity, [0.0; 4]);
}

#[test]
fn gpu_light_uniform_encodes_multiple_lights_per_type() {
    let mut scene = Scene::new();
    for index in 0..MAX_GPU_LIGHTS_PER_TYPE {
        scene
            .directional_light(
                DirectionalLight::default()
                    .with_color(Color::from_linear_rgb(index as f32 + 0.1, 0.2, 0.3))
                    .with_illuminance_lux(1_000.0 + index as f32)
                    .with_shadows(index == 1),
            )
            .transform(Transform::default())
            .add()
            .expect("directional light inserts");
        scene
            .point_light(
                PointLight::default()
                    .with_color(Color::from_linear_rgb(0.1, index as f32 + 0.2, 0.3))
                    .with_intensity_candela(100.0 + index as f32)
                    .with_range(10.0 + index as f32),
            )
            .transform(Transform::at(Vec3::new(index as f32, 2.0, 3.0)))
            .add()
            .expect("point light inserts");
        scene
            .spot_light(
                SpotLight::default()
                    .with_color(Color::from_linear_rgb(0.1, 0.2, index as f32 + 0.3))
                    .with_intensity_candela(200.0 + index as f32)
                    .with_range(20.0 + index as f32)
                    .with_inner_cone_angle(Angle::from_degrees(10.0))
                    .with_outer_cone_angle(Angle::from_degrees(25.0)),
            )
            .transform(Transform::at(Vec3::new(1.0, index as f32, 3.0)))
            .add()
            .expect("spot light inserts");
    }
    let environment = PreparedEnvironmentLighting::default();

    let uniform = collect_gpu_light_uniform(&scene, Vec3::ZERO, &environment, false);

    assert_eq!(uniform.light_counts, [16.0, 16.0, 16.0, 0.0]);
    assert_eq!(uniform.directional_light_color[1], [1.1, 0.2, 0.3, 0.0]);
    assert_eq!(uniform.directional_shadow_control[1][0], 1.0);
    assert_eq!(uniform.directional_light_color[3], [3.1, 0.2, 0.3, 0.0]);
    assert_eq!(
        uniform.point_light_position_intensity[2][0..3],
        [2.0, 2.0, 3.0]
    );
    assert_eq!(uniform.point_light_color_range[2], [0.1, 2.2, 0.3, 12.0]);
    assert_eq!(
        uniform.spot_light_position_intensity[2][0..3],
        [1.0, 2.0, 3.0]
    );
    assert_eq!(uniform.spot_light_color_range[2], [0.1, 0.2, 2.3, 0.0]);
    assert!(uniform.spot_light_cone_range[2][0] > uniform.spot_light_cone_range[2][1]);
}

#[test]
fn gpu_lighting_stats_accept_many_point_lights_for_tiled_assignment() {
    let mut scene = Scene::new();
    for index in 0..=MAX_GPU_LIGHTS_PER_TYPE {
        scene
            .point_light(PointLight::default().with_intensity_candela(100.0))
            .transform(Transform::at(Vec3::new(index as f32, 0.0, 1.0)))
            .add()
            .expect("point light inserts");
    }

    super::super::collect_lighting_stats(&scene, crate::diagnostics::Backend::HeadlessGpu)
        .expect("GPU lighting stats must allow many point lights for tiled assignment");
    assert!(
        super::super::collect_lighting_stats(&scene, crate::diagnostics::Backend::Headless).is_ok(),
        "the CPU path can shade all point lights and should not inherit the GPU uniform cap"
    );
}

#[test]
fn area_lights_use_separate_gpu_capacity_from_point_lights() {
    let mut scene = Scene::new();
    scene
        .area_light(
            AreaLight::default()
                .with_luminous_flux_lumens(400.0)
                .with_shape(AreaLightShape::rect(2.0, 1.0)),
        )
        .transform(Transform::default())
        .add()
        .expect("area light inserts");
    scene
        .point_light(PointLight::default().with_intensity_candela(75.0))
        .transform(Transform::at(Vec3::new(0.0, 1.0, 1.5)))
        .add()
        .expect("point light inserts");

    super::super::collect_lighting_stats(&scene, crate::diagnostics::Backend::HeadlessGpu)
        .expect("one area light plus one point light must not exceed the point-light cap");
    let uniform = collect_gpu_light_uniform(
        &scene,
        Vec3::ZERO,
        &PreparedEnvironmentLighting::default(),
        false,
    );

    assert_eq!(uniform.light_counts, [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(
        uniform.point_light_position_intensity[0][0..3],
        [0.0, 1.0, 1.5]
    );
    assert!(
        uniform.area_light_position_flux[0][3] > 0.0,
        "area light carries luminous flux in the area-light uniform lane"
    );
}

#[test]
fn area_light_shape_encodes_gpu_visible_area_lane() {
    let mut scene = Scene::new();
    scene
        .area_light(
            AreaLight::default()
                .with_luminous_flux_lumens(400.0)
                .with_shape(AreaLightShape::rect(2.0, 1.0)),
        )
        .transform(Transform::default())
        .add()
        .expect("area light inserts");
    let environment = PreparedEnvironmentLighting::default();

    let uniform = collect_gpu_light_uniform(&scene, Vec3::ZERO, &environment, false);

    assert_eq!(uniform.light_counts, [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(
        uniform.point_light_position_intensity,
        [[0.0; 4]; MAX_GPU_LIGHTS_PER_TYPE]
    );
    assert_eq!(uniform.area_light_position_flux[0], [0.0, 0.0, 0.0, 400.0]);
    assert_eq!(uniform.area_light_axis_x_shape[0], [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(uniform.area_light_axis_y_range[0], [0.0, 0.5, 0.0, 0.0]);
    assert_eq!(uniform.area_light_color[0], [1.0, 1.0, 1.0, 0.0]);
}
