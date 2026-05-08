#![cfg(not(target_arch = "wasm32"))]

use scena::{
    Angle, AssetError, Assets, Backend, Capabilities, CapabilityStatus, ClippingPlane,
    ClippingPlaneSet, Color, DepthRange, DiagnosticCode, DiagnosticSeverity, DirectionalLight,
    EnvironmentSourceKind, GeometryDesc, GeometryTopology, Light, MaterialDesc, NodeKind,
    OrthographicCamera, PerspectiveCamera, PointLight, PrepareError, Primitive, RenderMode,
    Renderer, RendererOptions, Scene, SpotLight, Transform, Vec3, Vertex,
};

const CAMERA_DISTANCE_FOR_NDC_FIXTURES: f32 = 1.732_050_8;

fn ndc_fixture_camera_transform() -> Transform {
    Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES))
}

fn pixel_at(frame: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * width + x) * 4) as usize;
    frame[offset..offset + 4]
        .try_into()
        .expect("pixel slice has four channels")
}

fn split_screen_fxaa_scene() -> Scene {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            ndc_fixture_camera_transform(),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    let white_left_half = [
        Primitive::triangle([
            Vertex {
                position: Vec3::new(-1.0, -1.0, 0.0),
                color: Color::WHITE,
            },
            Vertex {
                position: Vec3::new(0.0, -1.0, 0.0),
                color: Color::WHITE,
            },
            Vertex {
                position: Vec3::new(0.0, 1.0, 0.0),
                color: Color::WHITE,
            },
        ]),
        Primitive::triangle([
            Vertex {
                position: Vec3::new(-1.0, -1.0, 0.0),
                color: Color::WHITE,
            },
            Vertex {
                position: Vec3::new(0.0, 1.0, 0.0),
                color: Color::WHITE,
            },
            Vertex {
                position: Vec3::new(-1.0, 1.0, 0.0),
                color: Color::WHITE,
            },
        ]),
    ];
    scene
        .add_renderable(scene.root(), white_left_half.to_vec(), Transform::default())
        .expect("split-screen primitive inserts");
    scene
}

fn fullscreen_white_scene() -> Scene {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            ndc_fixture_camera_transform(),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-1.0, -1.0, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(3.0, -1.0, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(-1.0, 3.0, 0.0),
                    color: Color::WHITE,
                },
            ])],
            Transform::default(),
        )
        .expect("fullscreen primitive inserts");
    scene
}

fn fullscreen_triangle_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            scena::GeometryVertex {
                position: Vec3::new(-1.0, -1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(3.0, -1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(-1.0, 3.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2],
    )
    .expect("fullscreen test geometry is valid")
}

fn shadow_receiver_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            scena::GeometryVertex {
                position: Vec3::new(-0.15, -0.18, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.15, -0.18, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.15, 0.18, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(-0.15, 0.18, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
    .expect("shadow receiver geometry is valid")
}

fn shadow_caster_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            scena::GeometryVertex {
                position: Vec3::new(-0.23, -0.24, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.23, -0.24, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.23, 0.24, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(-0.23, 0.24, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
    .expect("shadow caster geometry is valid")
}

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
fn direct_lights_tint_pbr_mesh_output() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            ndc_fixture_camera_transform(),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    scene
        .directional_light(
            DirectionalLight::default()
                .with_color(Color::from_linear_rgb(1.0, 0.0, 0.0))
                .with_illuminance_lux(10_000.0),
        )
        .add()
        .expect("red directional light inserts");
    scene.mesh(geometry, material).add().expect("mesh inserts");

    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("lit mesh prepares");
    renderer
        .render_active(&scene)
        .expect("lit mesh renders through active camera");

    let pixel = pixel_at(renderer.frame_rgba8(), 8, 4, 4);
    assert!(
        pixel[0] > 100 && pixel[1] <= 1 && pixel[2] <= 2 && pixel[3] == 255,
        "direct light should produce red-dominant PBR preview output, got {pixel:?}",
    );
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
fn directional_shadow_receiver_pixels_are_darkened_by_caster() {
    fn render_shadow_fixture(with_caster: bool) -> [u8; 4] {
        let assets = Assets::new();
        let receiver = assets.create_geometry(shadow_receiver_geometry());
        let caster = assets.create_geometry(shadow_caster_geometry());
        let receiver_material =
            assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
        let caster_material = assets.create_material(MaterialDesc::unlit(Color::BLACK));
        let mut scene = Scene::new();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(0.0, 0.0, 3.0)),
            )
            .expect("camera inserts");
        scene
            .set_active_camera(camera)
            .expect("camera becomes active");
        scene
            .directional_light(
                DirectionalLight::default()
                    .with_illuminance_lux(10_000.0)
                    .with_shadows(true),
            )
            .transform(Transform::IDENTITY.rotate_y_deg(30.0))
            .add()
            .expect("shadowed directional light inserts");
        scene
            .mesh(receiver, receiver_material)
            .add()
            .expect("receiver inserts");
        if with_caster {
            scene
                .mesh(caster, caster_material)
                .transform(Transform::at(Vec3::new(0.29, 0.0, 0.50)))
                .add()
                .expect("caster inserts");
        }

        let mut renderer = Renderer::headless(80, 80).expect("renderer builds");
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("shadow fixture prepares");
        renderer
            .render_active(&scene)
            .expect("shadow fixture renders");
        pixel_at(renderer.frame_rgba8(), 80, 40, 40)
    }

    let lit_center = render_shadow_fixture(false);
    let shadowed_center = render_shadow_fixture(true);

    assert_eq!(shadowed_center[3], 255);
    assert!(
        shadowed_center[0] + 30 < lit_center[0]
            && shadowed_center[1] + 30 < lit_center[1]
            && shadowed_center[2] + 30 < lit_center[2],
        "shadowed receiver center should be visibly darker than the unshadowed receiver; lit={lit_center:?} shadowed={shadowed_center:?}",
    );
}

#[test]
fn headless_gpu_directional_shadow_visibility_darkens_receiver_when_available() {
    fn render_shadow_fixture(with_caster: bool) -> Option<[u8; 4]> {
        let assets = Assets::new();
        let receiver = assets.create_geometry(shadow_receiver_geometry());
        let caster = assets.create_geometry(shadow_caster_geometry());
        let receiver_material =
            assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
        let caster_material = assets.create_material(MaterialDesc::unlit(Color::BLACK));
        let mut scene = Scene::new();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(0.0, 0.0, 3.0)),
            )
            .expect("camera inserts");
        scene
            .set_active_camera(camera)
            .expect("camera becomes active");
        scene
            .directional_light(
                DirectionalLight::default()
                    .with_illuminance_lux(10_000.0)
                    .with_shadows(true),
            )
            .transform(Transform::IDENTITY.rotate_y_deg(30.0))
            .add()
            .expect("shadowed directional light inserts");
        scene
            .mesh(receiver, receiver_material)
            .add()
            .expect("receiver inserts");
        if with_caster {
            scene
                .mesh(caster, caster_material)
                .transform(Transform::at(Vec3::new(0.29, 0.0, 0.50)))
                .add()
                .expect("caster inserts");
        }

        let mut renderer = Renderer::headless_gpu(80, 80).ok()?;
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("GPU shadow fixture prepares");
        renderer
            .render_active(&scene)
            .expect("GPU shadow fixture renders");
        Some(pixel_at(renderer.frame_rgba8(), 80, 40, 40))
    }

    let Some(lit_center) = render_shadow_fixture(false) else {
        return;
    };
    let Some(shadowed_center) = render_shadow_fixture(true) else {
        return;
    };

    assert_eq!(shadowed_center[3], 255);
    assert!(
        shadowed_center[0] + 20 < lit_center[0]
            && shadowed_center[1] + 20 < lit_center[1]
            && shadowed_center[2] + 20 < lit_center[2],
        "prepared GPU shadow visibility should visibly darken the receiver; lit={lit_center:?} shadowed={shadowed_center:?}",
    );
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
fn depth_prepass_is_skipped_for_trivial_single_primitive_scene() {
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
    assert_eq!(prepared.depth_prepass_passes, 0);
    assert_eq!(prepared.depth_prepass_draws, 0);

    let mut empty_scene = Scene::new();
    renderer
        .prepare(&mut empty_scene)
        .expect("empty scene prepares");
    let released = renderer.stats();
    assert_eq!(released.depth_prepass_passes, 0);
    assert_eq!(released.depth_prepass_draws, 0);
}

#[test]
fn depth_prepass_is_prepared_when_multiple_opaque_primitives_benefit() {
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle(), Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("opaque primitives insert");
    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");

    renderer.prepare(&mut scene).expect("scene prepares");
    let prepared = renderer.stats();

    assert_eq!(prepared.depth_prepass_passes, 1);
    assert_eq!(prepared.depth_prepass_draws, 2);
}

#[test]
fn cpu_depth_buffer_keeps_nearer_triangle_visible_when_submitted_first() {
    let mut scene = depth_overlap_scene();

    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer.render_active(&scene).expect("scene renders");

    let center = pixel_at(renderer.frame_rgba8(), 64, 32, 32);
    assert!(
        center[1] > center[0],
        "near green triangle must win the depth test over later far red triangle, center={center:?}"
    );
}

#[test]
fn headless_gpu_depth_buffer_keeps_nearer_triangle_visible_when_available() {
    let mut scene = depth_overlap_scene();
    let Ok(mut renderer) = Renderer::headless_gpu(64, 64) else {
        return;
    };

    renderer
        .prepare(&mut scene)
        .expect("gpu depth-overlap scene prepares");
    renderer
        .render_active(&scene)
        .expect("gpu depth-overlap scene renders");

    let center = pixel_at(renderer.frame_rgba8(), 64, 32, 32);
    assert!(
        center[1] > center[0],
        "near green triangle must win the native GPU depth test over later far red triangle, center={center:?}"
    );
}

#[test]
fn near_far_precision_fixture_keeps_depth_order_for_small_and_large_scenes() {
    for (origin_shift, object_translation) in [
        (Vec3::ZERO, Vec3::ZERO),
        (Vec3::new(10_000.0, 0.0, 0.0), Vec3::new(10_000.0, 0.0, 0.0)),
    ] {
        let mut scene = precision_depth_scene(origin_shift, object_translation);
        let mut renderer = Renderer::headless(64, 64).expect("renderer builds");

        renderer
            .prepare(&mut scene)
            .expect("precision scene prepares");
        renderer
            .render_active(&scene)
            .expect("precision scene renders");

        let center = pixel_at(renderer.frame_rgba8(), 64, 32, 32);
        assert!(
            center[1] > center[0] && center[1] > center[2],
            "near green triangle must win depth order across the near/far precision fixture, center={center:?}"
        );
    }
}

fn depth_overlap_scene() -> Scene {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-0.5, -0.5, 0.0),
                    color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.5, -0.5, 0.0),
                    color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 0.5, 0.0),
                    color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                },
            ])],
            Transform::default(),
        )
        .expect("near triangle inserts first");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-0.5, -0.5, -0.5),
                    color: Color::from_linear_rgb(1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.5, -0.5, -0.5),
                    color: Color::from_linear_rgb(1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 0.5, -0.5),
                    color: Color::from_linear_rgb(1.0, 0.0, 0.0),
                },
            ])],
            Transform::default(),
        )
        .expect("far triangle inserts second");

    scene
}

fn precision_depth_scene(origin_shift: Vec3, object_translation: Vec3) -> Scene {
    let mut scene = Scene::new();
    scene.set_origin_shift(origin_shift);
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_depth_range(DepthRange::new(0.001, 5_000.0)),
            ndc_fixture_camera_transform(),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-0.8, -0.8, 0.0),
                    color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.8, -0.8, 0.0),
                    color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 0.8, 0.0),
                    color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                },
            ])],
            Transform::at(object_translation),
        )
        .expect("near precision triangle inserts first");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-0.8, -0.8, -0.2),
                    color: Color::from_linear_rgb(1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.8, -0.8, -0.2),
                    color: Color::from_linear_rgb(1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 0.8, -0.2),
                    color: Color::from_linear_rgb(1.0, 0.0, 0.0),
                },
            ])],
            Transform::at(object_translation),
        )
        .expect("far precision triangle inserts second");

    scene
}

#[test]
fn fxaa_pass_runs_after_aces_without_second_tonemap() {
    let mut scene = split_screen_fxaa_scene();
    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");

    renderer.prepare(&mut scene).expect("scene prepares");
    renderer
        .render_active(&scene)
        .expect("active camera renders split-screen fixture");

    let stats = renderer.stats();
    assert_eq!(stats.fxaa_passes, 1);
    assert_eq!(
        pixel_at(renderer.frame_rgba8(), 8, 1, 4),
        [206, 206, 206, 255]
    );
    assert_eq!(pixel_at(renderer.frame_rgba8(), 8, 6, 4), [0, 0, 0, 255]);

    let left_edge = pixel_at(renderer.frame_rgba8(), 8, 3, 4);
    let right_edge = pixel_at(renderer.frame_rgba8(), 8, 4, 4);
    assert_eq!(
        left_edge,
        [206, 206, 206, 255],
        "FXAA keeps bright edge pixels at ACES white instead of tonemapping twice"
    );
    assert!(
        right_edge[0] > 0,
        "FXAA smooths the dark edge pixel without changing solid black"
    );
}

#[test]
fn exposure_change_rerenders_on_change_and_changes_nonflat_pixels() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            ndc_fixture_camera_transform(),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-0.8, -0.8, 0.0),
                    color: Color::from_linear_rgb(0.18, 0.18, 0.18),
                },
                Vertex {
                    position: Vec3::new(0.8, -0.8, 0.0),
                    color: Color::from_linear_rgb(0.18, 0.18, 0.18),
                },
                Vertex {
                    position: Vec3::new(0.0, 0.8, 0.0),
                    color: Color::from_linear_rgb(0.18, 0.18, 0.18),
                },
            ])],
            Transform::default(),
        )
        .expect("gray triangle inserts");
    let mut renderer = Renderer::headless_with_options(
        32,
        32,
        RendererOptions::default().with_render_mode(RenderMode::OnChange),
    )
    .expect("renderer builds");

    renderer.prepare(&mut scene).expect("scene prepares");
    let first = renderer.render_active(&scene).expect("first frame renders");
    assert!(!first.skipped);
    let before = pixel_at(renderer.frame_rgba8(), 32, 16, 16);
    let skipped = renderer
        .render_active(&scene)
        .expect("unchanged frame skips");
    assert!(skipped.skipped);

    renderer.set_exposure_ev(2.0);
    let after_exposure = renderer
        .render_active(&scene)
        .expect("exposure change renders without reprepare");
    assert!(
        !after_exposure.skipped,
        "exposure changes are renderer-output changes and must not be skipped in OnChange mode"
    );
    let after = pixel_at(renderer.frame_rgba8(), 32, 16, 16);
    assert!(
        after[0] > before[0],
        "positive exposure should brighten non-flat rendered content, before={before:?}, after={after:?}"
    );
}

#[test]
fn prepare_emits_structured_depth_precision_warnings() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_depth_range(DepthRange::new(0.001, 200.0)),
            Transform::default(),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    scene
        .add_empty(
            scene.root(),
            Transform {
                translation: Vec3::new(10_000.0, 0.0, 0.0),
                ..Transform::default()
            },
        )
        .expect("large-offset node inserts");
    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");

    renderer.prepare(&mut scene).expect("scene prepares");
    let diagnostics = renderer.diagnostics();

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::DepthPrecisionRisk
            && diagnostic.severity == DiagnosticSeverity::Warning
            && diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help.contains("fit_sphere"))
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::LargeScenePrecisionRisk
            && diagnostic.severity == DiagnosticSeverity::Warning
            && diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help.contains("camera-relative"))
    }));
}

#[test]
fn capability_matrix_reports_reversed_z_depth_support_and_webgl2_fallback() {
    assert_eq!(
        Capabilities::for_gpu_backend(Backend::HeadlessGpu).reversed_z_depth,
        CapabilityStatus::Supported
    );
    assert_eq!(
        Capabilities::for_attached_gpu_backend(Backend::WebGpu).reversed_z_depth,
        CapabilityStatus::Supported
    );
    assert_eq!(
        Capabilities::for_attached_gpu_backend(Backend::WebGl2).reversed_z_depth,
        CapabilityStatus::FeatureDisabled
    );
}

#[test]
fn webgl2_depth_capability_reports_structured_compatibility_warning() {
    let diagnostics = Capabilities::for_attached_gpu_backend(Backend::WebGl2).diagnostics();

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::WebGl2DepthCompatibility
            && diagnostic.severity == DiagnosticSeverity::Warning
            && diagnostic.message.contains("WebGL2")
            && diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help.contains("near/far"))
    }));
}

#[test]
fn clipping_plane_set_clips_rendered_output_half_space() {
    let mut scene = fullscreen_white_scene();
    let plane = scene.add_clipping_plane(ClippingPlane::new(Vec3::new(1.0, 0.0, 0.0), 0.0));
    scene
        .set_clipping_planes(ClippingPlaneSet::new().with_plane(plane))
        .expect("active clipping plane set is valid");
    let mut renderer = Renderer::headless(16, 16).expect("renderer builds");

    renderer.prepare(&mut scene).expect("scene prepares");
    renderer
        .render_active(&scene)
        .expect("clipped scene renders");

    assert_eq!(pixel_at(renderer.frame_rgba8(), 16, 3, 8), [0, 0, 0, 255]);
    assert_eq!(
        pixel_at(renderer.frame_rgba8(), 16, 12, 8),
        [206, 206, 206, 255]
    );
}

#[test]
fn origin_shift_keeps_large_offset_renderable_visible_without_precision_warning() {
    let mut scene = Scene::new();
    scene.set_origin_shift(Vec3::new(10_000.0, 0.0, 0.0));
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            ndc_fixture_camera_transform(),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-1.0, -1.0, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(3.0, -1.0, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(-1.0, 3.0, 0.0),
                    color: Color::WHITE,
                },
            ])],
            Transform {
                translation: Vec3::new(10_000.0, 0.0, 0.0),
                ..Transform::default()
            },
        )
        .expect("large-offset renderable inserts");
    let mut renderer = Renderer::headless(8, 8).expect("renderer builds");

    renderer.prepare(&mut scene).expect("scene prepares");
    renderer
        .render_active(&scene)
        .expect("origin-shifted renderable renders");

    assert_eq!(
        pixel_at(renderer.frame_rgba8(), 8, 4, 4),
        [206, 206, 206, 255]
    );
    assert!(
        !renderer
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::LargeScenePrecisionRisk)
    );
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
            vec![Primitive::unlit_triangle(), Primitive::unlit_triangle()],
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
    assert_eq!(prepared.depth_prepass_draws, 2);

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
