#![cfg(not(target_arch = "wasm32"))]

use scena::{
    Angle, AssetError, Assets, Color, DepthRange, DirectionalLight, EnvironmentSourceKind, Light,
    NodeKind, OrthographicCamera, PerspectiveCamera, PointLight, PrepareError, Primitive, Renderer,
    Scene, SpotLight, Transform, Vec3,
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

#[test]
fn camera_depth_fit_helpers_cover_unit_cube_reference_distances() {
    let unit_cube_radius = 3.0_f32.sqrt() * 0.5;

    for center_distance in [1.0, 100.0, 10_000.0] {
        let range = DepthRange::fit_sphere(center_distance, unit_cube_radius);
        assert!(range.near() > 0.0);
        assert!(range.far() > range.near());
        assert!(range.contains_interval(
            center_distance - unit_cube_radius,
            center_distance + unit_cube_radius
        ));

        let perspective = PerspectiveCamera::default().with_depth_range(range);
        assert_eq!(perspective.near, range.near());
        assert_eq!(perspective.far, range.far());

        let orthographic = OrthographicCamera::default().with_depth_range(range);
        assert_eq!(orthographic.near, range.near());
        assert_eq!(orthographic.far, range.far());
    }

    let fallback = DepthRange::fit_sphere(f32::NAN, -1.0);
    assert_eq!(fallback, DepthRange::new(0.01, 1000.0));
}

#[test]
fn shadowed_directional_light_is_opt_in_and_single_owner() {
    assert!(!DirectionalLight::default().casts_shadows());
    assert!(
        DirectionalLight::default()
            .with_shadows(true)
            .casts_shadows()
    );

    let mut single_scene = Scene::new();
    single_scene
        .directional_light(DirectionalLight::default().with_shadows(true))
        .add()
        .expect("first shadowed directional light inserts");
    Renderer::headless(4, 4)
        .expect("renderer builds")
        .prepare(&mut single_scene)
        .expect("one shadowed directional light is allowed");

    let mut scene = Scene::new();
    let first = scene
        .directional_light(DirectionalLight::default().with_shadows(true))
        .add()
        .expect("first shadowed directional light inserts");
    let second = scene
        .directional_light(DirectionalLight::default().with_shadows(true))
        .add()
        .expect("second shadowed directional light inserts");

    let error = Renderer::headless(4, 4)
        .expect("renderer builds")
        .prepare(&mut scene)
        .expect_err("second shadowed directional light is rejected");

    assert_eq!(
        error,
        PrepareError::MultipleShadowedDirectionalLights { first, second }
    );
}

#[test]
fn single_shadow_map_records_pcf3x3_prepare_stats() {
    let mut scene = Scene::new();
    scene
        .directional_light(DirectionalLight::default().with_shadows(true))
        .add()
        .expect("shadowed directional light inserts");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("shadow caster inserts");

    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");
    assert_eq!(
        renderer.capabilities().directional_shadow_map_default_size,
        2048
    );
    assert_eq!(renderer.capabilities().directional_shadow_pcf_kernel, 3);

    renderer
        .prepare(&mut scene)
        .expect("one shadow map prepares");
    let stats = renderer.stats();

    assert_eq!(stats.shadow_maps, 1);
    assert_eq!(stats.directional_shadow_map_resolution, Some(2048));
    assert_eq!(stats.directional_shadow_pcf_kernel, Some(3));
}

#[test]
fn equirectangular_hdr_environment_loading_records_source_contract() {
    let assets = Assets::new();
    let environment =
        pollster::block_on(assets.load_environment("tests/assets/environment/studio_1024x512.hdr"))
            .expect("equirectangular HDR environment loads");
    let duplicate =
        pollster::block_on(assets.load_environment("tests/assets/environment/studio_1024x512.hdr"))
            .expect("duplicate equirectangular HDR environment loads");
    assert_eq!(environment, duplicate);

    let desc = assets
        .environment(environment)
        .expect("environment descriptor is stored");
    assert_eq!(
        desc.source_kind(),
        EnvironmentSourceKind::EquirectangularHdr
    );
    assert!(desc.is_equirectangular_hdr());
    assert_eq!(desc.source_dimensions(), Some((1024, 512)));
    assert_eq!(desc.cubemap_resolution(), 0);
    assert_eq!(desc.brdf_lut_size(), 0);
    assert!(desc.derivatives().is_empty());

    let error = pollster::block_on(assets.load_environment("tests/assets/environment/studio.exr"))
        .expect_err("unsupported environment format is rejected");
    assert_eq!(
        error,
        AssetError::UnsupportedEnvironmentFormat {
            path: "tests/assets/environment/studio.exr".to_string(),
            help: "use Radiance .hdr equirectangular input for the M2 environment path",
        }
    );
}

#[test]
fn equirectangular_environment_prepare_generates_ibl_resources() {
    let assets = Assets::new();
    let environment =
        pollster::block_on(assets.load_environment("tests/assets/environment/studio_1024x512.hdr"))
            .expect("equirectangular HDR environment loads");
    let mut scene = Scene::new();
    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");
    renderer.set_environment(environment);

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("equirectangular environment prepares");
    let stats = renderer.stats();

    assert_eq!(stats.environments, 1);
    assert_eq!(stats.environment_cubemaps, 1);
    assert_eq!(stats.environment_prefilter_passes, 1);
    assert_eq!(stats.environment_brdf_luts, 1);
}

#[test]
fn depth_prepass_is_prepared_for_opaque_scene_geometry() {
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("opaque primitive inserts");
    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");

    renderer.prepare(&mut scene).expect("scene prepares");
    let prepared = renderer.stats();
    assert_eq!(prepared.depth_prepass_passes, 1);
    assert_eq!(prepared.depth_prepass_draws, 1);

    let mut empty_scene = Scene::new();
    renderer
        .prepare(&mut empty_scene)
        .expect("empty scene prepares");
    let released = renderer.stats();
    assert_eq!(released.depth_prepass_passes, 0);
    assert_eq!(released.depth_prepass_draws, 0);
}

#[test]
fn m2_resource_counters_return_to_baseline_after_empty_prepare() {
    let assets = Assets::new();
    let environment =
        pollster::block_on(assets.load_environment("tests/assets/environment/studio_1024x512.hdr"))
            .expect("equirectangular HDR environment loads");
    let mut scene = Scene::new();
    scene
        .directional_light(DirectionalLight::default().with_shadows(true))
        .add()
        .expect("shadowed directional light inserts");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("opaque primitive inserts");
    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");
    let baseline = renderer.stats();

    renderer.set_environment(environment);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("M2 resources prepare");
    let prepared = renderer.stats();
    assert_eq!(prepared.environments, 1);
    assert_eq!(prepared.environment_cubemaps, 1);
    assert_eq!(prepared.environment_prefilter_passes, 1);
    assert_eq!(prepared.environment_brdf_luts, 1);
    assert_eq!(prepared.shadow_maps, 1);
    assert_eq!(prepared.directional_shadow_map_resolution, Some(2048));
    assert_eq!(prepared.directional_shadow_pcf_kernel, Some(3));
    assert_eq!(prepared.depth_prepass_passes, 1);
    assert_eq!(prepared.depth_prepass_draws, 1);

    renderer.clear_environment();
    let mut empty_scene = Scene::new();
    renderer
        .prepare(&mut empty_scene)
        .expect("empty scene prepares after clearing M2 resources");
    let released = renderer.stats();

    assert_eq!(released.environments, baseline.environments);
    assert_eq!(released.environment_cubemaps, baseline.environment_cubemaps);
    assert_eq!(
        released.environment_prefilter_passes,
        baseline.environment_prefilter_passes
    );
    assert_eq!(
        released.environment_brdf_luts,
        baseline.environment_brdf_luts
    );
    assert_eq!(released.shadow_maps, baseline.shadow_maps);
    assert_eq!(
        released.directional_shadow_map_resolution,
        baseline.directional_shadow_map_resolution
    );
    assert_eq!(
        released.directional_shadow_pcf_kernel,
        baseline.directional_shadow_pcf_kernel
    );
    assert_eq!(released.depth_prepass_passes, baseline.depth_prepass_passes);
    assert_eq!(released.depth_prepass_draws, baseline.depth_prepass_draws);
    assert_eq!(released.textures, baseline.textures);
    assert_eq!(released.pending_destructions, baseline.pending_destructions);
}
