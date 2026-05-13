#![cfg(not(target_arch = "wasm32"))]

use scena::{
    Aabb, AlphaMode, AlphaPipelineStatus, AssetPath, Assets, Backend, Capabilities, Color,
    DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES, DEFAULT_STROKE_WIDTH_PX, EnvironmentDesc,
    EnvironmentHandle, EnvironmentSourceKind, GeometryDesc, GeometryHandle, GeometryTopology,
    MaterialDesc, MaterialHandle, MaterialKind, ModelHandle, NodeKind, NotPreparedReason,
    OutputStageStatus, PerspectiveCamera, PrepareError, Primitive, RenderError, Renderer, Scene,
    SceneAsset, TextureColorSpace, TextureDesc, TextureHandle, Tonemapper, Transform, Vec3, Vertex,
    WasmEnvironmentDelivery,
};

const CAMERA_DISTANCE_FOR_NDC_FIXTURES: f32 = 1.732_050_8;

fn assert_handle<T: Copy + Eq + std::fmt::Debug>() {}

fn center_pixel(frame: &[u8], width: u32, height: u32) -> [u8; 4] {
    let x = width / 2;
    let y = height / 2;
    pixel_at(frame, width, x, y)
}

fn pixel_at(frame: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * width + x) * 4) as usize;
    frame[offset..offset + 4]
        .try_into()
        .expect("pixel slice has four channels")
}

fn unstable_headless_gpu_release_tests_enabled() -> bool {
    std::env::var_os("SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS").is_some()
}

fn record_fail_closed_headless_gpu_lane(test_name: &str, reason: &str) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/gate-artifacts/gpu-release-gaps");
    std::fs::create_dir_all(&dir).expect("gpu-release-gaps artifact dir");
    let artifact = serde_json::json!({
        "schema": "scena.gpu_release_gap.v1",
        "test_name": test_name,
        "status": "fail-closed",
        "release_evidence": false,
        "reason": reason,
        "run_hint": "Set SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1 on an approved visual lane to run the local headless-GPU assertion.",
    });
    std::fs::write(
        dir.join(format!("{test_name}.json")),
        serde_json::to_vec_pretty(&artifact).expect("gpu gap artifact serializes"),
    )
    .expect("gpu gap artifact writes");
}

fn assert_all_pixels(frame: &[u8], width: u32, height: u32, expected: [u8; 4]) {
    assert_eq!(frame.len(), (width as usize) * (height as usize) * 4);
    for (index, pixel) in frame.chunks_exact(4).enumerate() {
        assert_eq!(
            pixel, expected,
            "pixel {index} should match fullscreen constant-color output"
        );
    }
}

fn assert_pixel_close(actual: [u8; 4], expected: [u8; 4], tolerance: u8, context: &str) {
    for channel in 0..3 {
        assert!(
            actual[channel].abs_diff(expected[channel]) <= tolerance,
            "{context}: color channel {channel} should be within {tolerance} of expected \
             {expected:?}, got {actual:?}"
        );
    }
    assert_eq!(actual[3], expected[3], "{context}: alpha should match");
}

fn assert_all_pixels_close(
    frame: &[u8],
    width: u32,
    height: u32,
    expected: [u8; 4],
    tolerance: u8,
) {
    assert_eq!(frame.len(), (width as usize) * (height as usize) * 4);
    for (index, pixel) in frame.chunks_exact(4).enumerate() {
        let actual: [u8; 4] = pixel.try_into().expect("pixel slice has four channels");
        assert_pixel_close(
            actual,
            expected,
            tolerance,
            &format!("pixel {index} should match gpu output within backend tolerance"),
        );
    }
}

fn count_pixels_close(frame: &[u8], expected: [u8; 4], tolerance: u8) -> usize {
    frame
        .chunks_exact(4)
        .filter(|pixel| {
            pixel[3] == expected[3]
                && pixel[0].abs_diff(expected[0]) <= tolerance
                && pixel[1].abs_diff(expected[1]) <= tolerance
                && pixel[2].abs_diff(expected[2]) <= tolerance
        })
        .count()
}

fn assert_lower_hex_sha256(value: &str) {
    assert_eq!(value.len(), 64);
    assert!(
        value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
}

fn rendered_fullscreen_center_pixel(color: Color) -> [u8; 4] {
    let (mut scene, camera) = scene_with_fullscreen_triangle(color);
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");
    renderer.render(&scene, camera).expect("render succeeds");
    center_pixel(renderer.frame_rgba8(), 4, 4)
}

fn scene_with_fullscreen_triangle(color: Color) -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES)),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-2.0, -2.0, 0.0),
                    color,
                },
                Vertex {
                    position: Vec3::new(4.0, -2.0, 0.0),
                    color,
                },
                Vertex {
                    position: Vec3::new(-2.0, 4.0, 0.0),
                    color,
                },
            ])],
            Transform::default(),
        )
        .expect("fullscreen triangle inserts");
    (scene, camera)
}

fn scene_with_camera() -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES)),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");
    (scene, camera)
}

fn scene_with_fullscreen_primitives(primitives: Vec<Primitive>) -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES)),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");
    scene
        .add_renderable(scene.root(), primitives, Transform::default())
        .expect("fullscreen primitives insert");
    (scene, camera)
}

fn fullscreen_triangle(color: Color) -> Primitive {
    // This oversized triangle contains the whole NDC unit square after viewport mapping.
    Primitive::triangle([
        Vertex {
            position: Vec3::new(-2.0, -2.0, 0.0),
            color,
        },
        Vertex {
            position: Vec3::new(4.0, -2.0, 0.0),
            color,
        },
        Vertex {
            position: Vec3::new(-2.0, 4.0, 0.0),
            color,
        },
    ])
}

fn quad_primitives(x0: f32, y0: f32, x1: f32, y1: f32, color: Color) -> [Primitive; 2] {
    [
        Primitive::triangle([
            Vertex {
                position: Vec3::new(x0, y0, 0.0),
                color,
            },
            Vertex {
                position: Vec3::new(x1, y0, 0.0),
                color,
            },
            Vertex {
                position: Vec3::new(x1, y1, 0.0),
                color,
            },
        ]),
        Primitive::triangle([
            Vertex {
                position: Vec3::new(x0, y0, 0.0),
                color,
            },
            Vertex {
                position: Vec3::new(x1, y1, 0.0),
                color,
            },
            Vertex {
                position: Vec3::new(x0, y1, 0.0),
                color,
            },
        ]),
    ]
}

fn scene_with_checkerboard() -> (Scene, scena::CameraKey) {
    let mut primitives = Vec::new();
    primitives.extend(quad_primitives(-2.0, 0.0, 0.0, 2.0, Color::WHITE));
    primitives.extend(quad_primitives(0.0, 0.0, 2.0, 2.0, Color::BLACK));
    primitives.extend(quad_primitives(-2.0, -2.0, 0.0, 0.0, Color::BLACK));
    primitives.extend(quad_primitives(0.0, -2.0, 2.0, 0.0, Color::WHITE));
    scene_with_fullscreen_primitives(primitives)
}

fn fullscreen_triangle_geometry() -> GeometryDesc {
    fullscreen_triangle_geometry_at(0.0)
}

fn fullscreen_triangle_geometry_at(z: f32) -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            scena::GeometryVertex {
                position: Vec3::new(-2.0, -2.0, z),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(4.0, -2.0, z),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(-2.0, 4.0, z),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2],
    )
    .expect("fullscreen test geometry is valid")
}

fn flat_square_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            scena::GeometryVertex {
                position: Vec3::new(-0.75, -0.75, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.75, -0.75, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.75, 0.75, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(-0.75, 0.75, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
    .expect("flat square test geometry is valid")
}

#[test]
fn asset_taxonomy_reserves_distinct_typed_handles() {
    assert_handle::<ModelHandle>();
    assert_handle::<GeometryHandle>();
    assert_handle::<MaterialHandle>();
    assert_handle::<TextureHandle>();
    assert_handle::<EnvironmentHandle>();

    let scene_asset = SceneAsset::empty();
    assert!(format!("{scene_asset:?}").contains("SceneAsset"));
}

#[test]
fn default_environment_manifest_fields_are_structured_and_loadable() {
    let environment = EnvironmentDesc::neutral_studio();

    assert_eq!(environment.name(), "neutral-studio");
    assert_eq!(
        environment.source_path().as_str(),
        "tests/assets/environment/neutral-studio.fixture.txt"
    );
    assert_eq!(
        environment.source_kind(),
        EnvironmentSourceKind::BundledPreviewFixture
    );
    assert!(!environment.is_equirectangular_hdr());
    assert_lower_hex_sha256(
        environment
            .source_sha256()
            .expect("default environment pins source hash"),
    );
    assert_eq!(environment.license(), Some("CC0-1.0"));
    assert_eq!(
        environment.generator(),
        Some(
            "xtask generate-default-env-fixture --input tests/assets/environment/neutral-studio.fixture.txt"
        )
    );
    assert_eq!(environment.cubemap_resolution(), 256);
    assert_eq!(environment.brdf_lut_size(), 256);
    assert_eq!(
        environment.wasm_delivery(),
        WasmEnvironmentDelivery::Bundled
    );
    assert_eq!(environment.derivatives().len(), 2);
    assert_eq!(
        environment.derivatives()[0].path().as_str(),
        "tests/assets/environment/generated/neutral-studio-cubemap.fixture.toml"
    );
    assert_eq!(
        environment.derivatives()[1].path().as_str(),
        "tests/assets/environment/generated/brdf-lut-256.fixture.toml"
    );
    for derivative in environment.derivatives() {
        assert_lower_hex_sha256(derivative.sha256());
    }

    let assets = Assets::new();
    let default = assets.default_environment();
    assert_eq!(assets.environment(default), Some(environment.clone()));

    let loaded = pollster::block_on(assets.load_environment("environments/factory.hdr"))
        .expect("environment request is recorded");
    let duplicate = pollster::block_on(assets.load_environment("environments/factory.hdr"))
        .expect("duplicate environment request is recorded");
    assert_eq!(loaded, duplicate);
    assert_eq!(
        assets
            .environment(loaded)
            .expect("environment descriptor is stored")
            .source_path()
            .as_str(),
        "environments/factory.hdr"
    );

    let fresh_assets = Assets::new();
    let default_by_path =
        pollster::block_on(fresh_assets.load_environment(environment.source_path().as_str()))
            .expect("default environment path is recognized");
    assert_eq!(fresh_assets.environment(default_by_path), Some(environment));
    assert_eq!(fresh_assets.default_environment(), default_by_path);
}

#[test]
fn default_environment_derivatives_are_renderer_consumable_fixtures() {
    let environment = EnvironmentDesc::neutral_studio();
    let cubemap = &environment.derivatives()[0];
    let brdf_lut = &environment.derivatives()[1];

    assert!(!cubemap.path().as_str().contains("placeholder"));
    assert!(!brdf_lut.path().as_str().contains("placeholder"));

    let cubemap_payload = std::fs::read_to_string(cubemap.path().as_str())
        .expect("default cubemap derivative is committed")
        .replace("\r\n", "\n");
    assert!(cubemap_payload.starts_with("SCENA_CUBEMAP_V1\n"));
    assert!(cubemap_payload.contains("faces = 6\n"));
    assert!(cubemap_payload.contains("resolution = 256\n"));
    assert!(!cubemap_payload.contains("not a renderer-consumable"));

    let brdf_payload = std::fs::read_to_string(brdf_lut.path().as_str())
        .expect("BRDF LUT derivative is committed")
        .replace("\r\n", "\n");
    assert!(brdf_payload.starts_with("SCENA_BRDF_LUT_V1\n"));
    assert!(brdf_payload.contains("size = 256\n"));
    assert!(brdf_payload.contains("encoding = rgba16f-text-fixture\n"));
    assert!(!brdf_payload.contains("not a renderer-consumable"));
}

#[test]
fn m1_cpu_resource_lifetime_counters_return_to_baseline() {
    let baseline = Renderer::headless(4, 4)
        .expect("headless renderer builds")
        .stats();
    assert_eq!(baseline.buffers, 0);
    assert_eq!(baseline.textures, 0);
    assert_eq!(baseline.materials, 0);
    assert_eq!(baseline.render_targets, 0);
    assert_eq!(baseline.pipelines, 0);
    assert_eq!(baseline.bind_groups, 0);
    assert_eq!(baseline.shader_modules, 0);
    assert_eq!(baseline.environments, 0);
    assert_eq!(baseline.scene_imports, 0);
    assert_eq!(baseline.live_logical_handles, 0);
    assert_eq!(baseline.pending_destructions, 0);

    for _ in 0..10 {
        let assets = Assets::new();
        let _texture = pollster::block_on(
            assets.load_texture("textures/lifetime.png", TextureColorSpace::Srgb),
        )
        .expect("texture request is recorded");
        let _environment = assets.default_environment();
        let geometry = assets.create_geometry(fullscreen_triangle_geometry());
        let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let (mut scene, camera) = scene_with_camera();
        scene
            .mesh(geometry, material)
            .add()
            .expect("mesh node inserts");
        let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");

        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("asset mesh prepares");
        renderer.render(&scene, camera).expect("asset mesh renders");

        let prepared = renderer.stats();
        assert_eq!(prepared.buffers, baseline.buffers);
        assert_eq!(prepared.textures, baseline.textures);
        assert_eq!(prepared.materials, 1);
        assert_eq!(prepared.render_targets, baseline.render_targets);
        assert_eq!(prepared.pipelines, baseline.pipelines);
        assert_eq!(prepared.bind_groups, baseline.bind_groups);
        assert_eq!(prepared.shader_modules, baseline.shader_modules);
        assert_eq!(prepared.environments, baseline.environments);
        assert_eq!(prepared.scene_imports, baseline.scene_imports);
        assert_eq!(prepared.live_logical_handles, 2);
        assert_eq!(prepared.pending_destructions, baseline.pending_destructions);
        assert_eq!(prepared.approximate_gpu_memory_bytes, None);
        assert_eq!(prepared.gpu_frame_ms, None);

        let (mut empty_scene, _empty_camera) = scene_with_camera();
        renderer
            .prepare(&mut empty_scene)
            .expect("empty scene releases logical resources");
        let released = renderer.stats();
        assert_eq!(released.buffers, baseline.buffers);
        assert_eq!(released.textures, baseline.textures);
        assert_eq!(released.materials, baseline.materials);
        assert_eq!(released.render_targets, baseline.render_targets);
        assert_eq!(released.pipelines, baseline.pipelines);
        assert_eq!(released.bind_groups, baseline.bind_groups);
        assert_eq!(released.shader_modules, baseline.shader_modules);
        assert_eq!(released.environments, baseline.environments);
        assert_eq!(released.scene_imports, baseline.scene_imports);
        assert_eq!(released.live_logical_handles, baseline.live_logical_handles);
        assert_eq!(released.pending_destructions, baseline.pending_destructions);
        assert_eq!(released.approximate_gpu_memory_bytes, None);
        assert_eq!(released.gpu_frame_ms, None);
    }
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn m1_headless_gpu_resource_counters_return_to_baseline_after_empty_reprepare() {
    if let Ok(mut renderer) = Renderer::headless_gpu(4, 4) {
        let baseline = renderer.stats();
        assert_eq!(baseline.buffers, 0);
        assert_eq!(baseline.textures, 0);
        assert_eq!(baseline.render_targets, 0);
        assert_eq!(baseline.pipelines, 0);
        assert_eq!(baseline.bind_groups, 0);
        assert_eq!(baseline.shader_modules, 0);
        assert_eq!(baseline.pending_destructions, 0);
        assert_eq!(baseline.approximate_gpu_memory_bytes, None);

        let (mut scene, camera) = scene_with_fullscreen_triangle(Color::WHITE);
        renderer.prepare(&mut scene).expect("gpu scene prepares");
        let prepared = renderer.stats();
        assert!(prepared.buffers >= 3);
        assert_eq!(prepared.textures, baseline.textures);
        // The headless GPU path keeps an offscreen color attachment plus a depth target
        // when the prepare phase decides a depth pre-pass is worthwhile; trivial single-
        // primitive scenes fall back to a single render target, a single pipeline, and a
        // single shader module. Use lower-bound checks so the resource-lifetime contract
        // (counters return to baseline) stays the focus and the optional depth pre-pass
        // resources are accepted whether or not the heuristic chose to include them.
        assert!(prepared.render_targets >= 1 && prepared.render_targets <= 2);
        assert!(prepared.pipelines >= 1);
        assert!(prepared.bind_groups >= 1);
        assert!(prepared.shader_modules >= 1);
        assert_eq!(prepared.pending_destructions, 0);
        assert!(prepared.approximate_gpu_memory_bytes.unwrap_or_default() > 0);
        renderer.render(&scene, camera).expect("gpu scene renders");

        let (mut empty_scene, _empty_camera) = scene_with_camera();
        renderer
            .prepare(&mut empty_scene)
            .expect("empty gpu scene prepares and releases resources");
        let queued = renderer.stats();
        assert_eq!(queued.buffers, baseline.buffers);
        assert_eq!(queued.textures, baseline.textures);
        assert_eq!(queued.render_targets, baseline.render_targets);
        assert_eq!(queued.pipelines, baseline.pipelines);
        assert_eq!(queued.bind_groups, baseline.bind_groups);
        assert_eq!(queued.shader_modules, baseline.shader_modules);
        assert!(queued.pending_destructions > baseline.pending_destructions);
        assert_eq!(
            queued.approximate_gpu_memory_bytes,
            baseline.approximate_gpu_memory_bytes
        );

        let poll = renderer.poll_device();
        assert!(poll.gpu_polled);
        assert_eq!(
            poll.pending_destructions_before,
            queued.pending_destructions
        );
        assert_eq!(poll.destroyed_resources, queued.pending_destructions);
        assert_eq!(
            poll.pending_destructions_after,
            baseline.pending_destructions
        );

        let released = renderer.stats();
        assert_eq!(released.pending_destructions, baseline.pending_destructions);
        assert_eq!(
            released.approximate_gpu_memory_bytes,
            baseline.approximate_gpu_memory_bytes
        );
    }
}

#[test]
fn renderer_environment_is_structural_and_validated_during_prepare() {
    let assets = Assets::new();
    let environment = assets.default_environment();
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(geometry, material)
        .add()
        .expect("default-environment mesh inserts");
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");

    assert_eq!(renderer.environment(), None);
    renderer.set_environment(environment);
    assert_eq!(renderer.environment(), Some(environment));
    assert!(matches!(
        renderer.prepare(&mut scene),
        Err(PrepareError::EnvironmentAssetsRequired { environment: error_environment })
            if error_environment == environment
    ));

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("default environment validates during prepare");
    assert_eq!(renderer.stats().environments, 1);
    renderer.render(&scene, camera).expect("scene renders");
    // After the CPU IBL fallback fix (cubemap-derived scalar irradiance for
    // environments without preview_irradiance_rgb), the default environment
    // now contributes real radiance to diffuse PBR surfaces, slightly
    // desaturating ACES on a fully-white-irradiated white material.
    assert_pixel_close(
        center_pixel(renderer.frame_rgba8(), 4, 4),
        [202, 208, 218, 255],
        2,
        "default-environment + white PBR converges to roughly equal-luminance \
         tonemapped grey across channels",
    );

    let missing_environment = EnvironmentHandle::default();
    renderer.set_environment(missing_environment);
    assert!(matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::EnvironmentChanged { .. }
        })
    ));
    assert!(matches!(
        renderer.prepare_with_assets(&mut scene, &assets),
        Err(PrepareError::EnvironmentNotFound { environment: error_environment })
            if error_environment == missing_environment
    ));
}

#[test]
fn m1_logical_asset_resource_counters_return_to_baseline_after_empty_prepare() {
    let assets = Assets::new();
    let albedo = pollster::block_on(
        assets.load_texture("textures/lifetime-albedo.png", TextureColorSpace::Srgb),
    )
    .expect("albedo texture is recorded");
    let normal = pollster::block_on(
        assets.load_texture("textures/lifetime-normal.png", TextureColorSpace::Linear),
    )
    .expect("normal texture is recorded");
    let metallic_roughness = pollster::block_on(assets.load_texture(
        "textures/lifetime-metallic-roughness.png",
        TextureColorSpace::Linear,
    ))
    .expect("metallic-roughness texture is recorded");
    let occlusion = pollster::block_on(
        assets.load_texture("textures/lifetime-occlusion.png", TextureColorSpace::Linear),
    )
    .expect("occlusion texture is recorded");
    let emissive = pollster::block_on(
        assets.load_texture("textures/lifetime-emissive.png", TextureColorSpace::Srgb),
    )
    .expect("emissive texture is recorded");
    let environment = assets.default_environment();
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0)
            .with_base_color_texture(albedo)
            .with_normal_texture(normal)
            .with_metallic_roughness_texture(metallic_roughness)
            .with_occlusion_texture(occlusion)
            .with_emissive_texture(emissive),
    );
    let (mut scene, _camera) = scene_with_camera();
    scene
        .mesh(geometry, material)
        .add()
        .expect("textured material mesh inserts");
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");
    let baseline = renderer.stats();

    renderer.set_environment(environment);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("textured material prepares");
    let prepared = renderer.stats();
    assert_eq!(prepared.materials, 1);
    assert_eq!(prepared.textures, 5);
    assert_eq!(prepared.environments, 1);
    assert_eq!(prepared.live_logical_handles, 8);

    renderer.clear_environment();
    let (mut empty_scene, _empty_camera) = scene_with_camera();
    renderer
        .prepare(&mut empty_scene)
        .expect("empty scene prepares after clearing environment");
    let released = renderer.stats();
    assert_eq!(released.materials, baseline.materials);
    assert_eq!(released.textures, baseline.textures);
    assert_eq!(released.environments, baseline.environments);
    assert_eq!(released.live_logical_handles, baseline.live_logical_handles);
    assert_eq!(released.pending_destructions, baseline.pending_destructions);
}

#[test]
fn material_descriptor_defaults_are_explicit() {
    let material = MaterialDesc::default();

    assert_eq!(material.kind(), MaterialKind::PbrMetallicRoughness);
    assert_eq!(material.base_color(), Color::WHITE);
    assert_eq!(material.alpha_mode(), AlphaMode::Opaque);
    assert_eq!(material.emissive(), Color::BLACK);
    assert_eq!(material.emissive_strength(), 1.0);
    assert_eq!(material.metallic_factor(), 0.0);
    assert_eq!(material.roughness_factor(), 1.0);
    assert!(!material.double_sided());
    assert_eq!(material.base_color_texture(), None);
    assert_eq!(material.normal_texture(), None);
    assert_eq!(material.metallic_roughness_texture(), None);
    assert_eq!(material.occlusion_texture(), None);
    assert_eq!(material.emissive_texture(), None);
}

#[test]
fn color_constructors_make_source_color_space_explicit() {
    assert_eq!(
        Color::from_srgb_u8(255, 128, 0),
        Color::from_srgb(1.0, 128.0 / 255.0, 0.0)
    );
    assert_eq!(
        Color::from_hex_srgb("#ff8000").expect("valid hex color"),
        Color::from_srgb_u8(255, 128, 0)
    );
    assert!(Color::from_hex_srgb("ff8000").is_err());
}

#[test]
fn headless_output_stage_applies_pbr_neutral_srgb_and_exposure_without_reprepare() {
    let (mut scene, camera) = scene_with_fullscreen_triangle(Color::WHITE);
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");

    assert_eq!(
        renderer.capabilities().output_stage,
        OutputStageStatus::PbrNeutralSrgb
    );
    assert_eq!(
        renderer.capabilities().alpha_pipeline,
        AlphaPipelineStatus::LinearSourceOver
    );
    assert_eq!(renderer.tonemapper(), Tonemapper::PbrNeutral);
    assert_eq!(renderer.exposure_ev(), 0.0);

    renderer.render(&scene, camera).expect("render succeeds");
    assert_eq!(
        center_pixel(renderer.frame_rgba8(), 4, 4),
        [240, 240, 240, 255]
    );

    renderer.set_exposure_ev(2.0);
    renderer
        .render_active(&scene)
        .expect("exposure is a steady-state update");
    assert_eq!(
        center_pixel(renderer.frame_rgba8(), 4, 4),
        [253, 253, 253, 255]
    );
}

#[test]
fn rendered_cpu_checkerboard_neutral_and_color_checker_samples_are_pinned() {
    let (mut checkerboard, camera) = scene_with_checkerboard();
    let mut renderer = Renderer::headless(8, 8).expect("headless renderer builds");
    renderer
        .prepare(&mut checkerboard)
        .expect("checkerboard prepares");
    renderer
        .render(&checkerboard, camera)
        .expect("checkerboard renders");
    // FXAA blurs the BLACK/WHITE quad boundary; the exact pixel values at single-pixel
    // distance from the seam are sensitive to floating-point rasterizer edge tests, which
    // differ between aarch64 (Pi) and x86_64 (CI runners). Use generous tolerance so the
    // contract is "WHITE quadrant looks bright, BLACK quadrant looks dark" instead of an
    // exact-pinned trio.
    assert_pixel_close(
        pixel_at(renderer.frame_rgba8(), 8, 2, 2),
        [133, 133, 133, 255],
        16,
        "top-left WHITE quadrant pixel",
    );
    assert_pixel_close(
        pixel_at(renderer.frame_rgba8(), 8, 6, 2),
        [34, 34, 34, 255],
        80,
        "top-right BLACK quadrant pixel within FXAA blur tolerance",
    );
    assert_pixel_close(
        pixel_at(renderer.frame_rgba8(), 8, 2, 6),
        [34, 34, 34, 255],
        80,
        "bottom-left BLACK quadrant pixel within FXAA blur tolerance",
    );
    assert_pixel_close(
        pixel_at(renderer.frame_rgba8(), 8, 6, 6),
        [240, 240, 240, 255],
        16,
        "bottom-right WHITE quadrant pixel",
    );

    assert_eq!(
        rendered_fullscreen_center_pixel(Color::BLACK),
        [0, 0, 0, 255]
    );
    assert_eq!(
        rendered_fullscreen_center_pixel(Color::WHITE),
        [240, 240, 240, 255]
    );
    assert_eq!(
        rendered_fullscreen_center_pixel(Color::from_linear_rgb(0.18, 0.18, 0.18)),
        [105, 105, 105, 255]
    );
    assert_eq!(
        rendered_fullscreen_center_pixel(Color::from_srgb(0.5, 0.5, 0.5)),
        [116, 116, 116, 255]
    );

    let color_checker = [
        (
            Color::from_linear_rgb(0.436, 0.246, 0.164),
            [169, 125, 99, 255],
        ),
        (
            Color::from_linear_rgb(0.051, 0.101, 0.411),
            [34, 73, 165, 255],
        ),
        (
            Color::from_linear_rgb(0.063, 0.239, 0.088),
            [44, 124, 63, 255],
        ),
    ];
    for (color, expected) in color_checker {
        assert_eq!(rendered_fullscreen_center_pixel(color), expected);
    }
}

#[test]
fn headless_alpha_blends_in_linear_before_output_encoding() {
    let (mut scene, camera) = scene_with_fullscreen_primitives(vec![
        fullscreen_triangle(Color::from_linear_rgba(0.0, 0.0, 1.0, 1.0)),
        fullscreen_triangle(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.5)),
    ]);
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");

    renderer.render(&scene, camera).expect("render succeeds");

    assert_all_pixels(renderer.frame_rgba8(), 4, 4, [188, 0, 188, 255]);
}

#[test]
fn prepare_with_assets_renders_scene_mesh_unlit_geometry() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(1.0, 0.0, 0.0)));
    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(geometry, material)
        .add()
        .expect("mesh node inserts");
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("asset-backed mesh prepares");
    renderer.render(&scene, camera).expect("render succeeds");

    assert_all_pixels(renderer.frame_rgba8(), 4, 4, [241, 33, 33, 255]);
}

#[test]
fn prepare_with_assets_sorts_blend_meshes_back_to_front_before_render() {
    let assets = Assets::new();
    let background = assets.create_geometry(fullscreen_triangle_geometry_at(0.0));
    let near = assets.create_geometry(fullscreen_triangle_geometry_at(0.8));
    let far = assets.create_geometry(fullscreen_triangle_geometry_at(0.2));
    let blue = assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.0, 0.0, 1.0)));
    let red_blend = assets.create_material(
        MaterialDesc::unlit(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.5))
            .with_alpha_mode(AlphaMode::Blend),
    );
    let green_blend = assets.create_material(
        MaterialDesc::unlit(Color::from_linear_rgba(0.0, 1.0, 0.0, 0.5))
            .with_alpha_mode(AlphaMode::Blend),
    );
    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(background, blue)
        .add()
        .expect("background mesh inserts");
    scene
        .mesh(near, red_blend)
        .add()
        .expect("near transparent mesh inserts first");
    scene
        .mesh(far, green_blend)
        .add()
        .expect("far transparent mesh inserts second");
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("blend meshes prepare");
    renderer.render(&scene, camera).expect("render succeeds");

    // Sorted draw order is blue opaque -> far green 50% -> near red 50%, giving linear
    // (0.5, 0.25, 0.25, 1.0) before PBR Neutral+sRGB output encoding.
    assert_all_pixels(renderer.frame_rgba8(), 4, 4, [181, 126, 126, 255]);
}

#[test]
fn prepare_with_assets_sorts_blend_meshes_by_camera_space_depth() {
    let assets = Assets::new();
    let background = assets.create_geometry(fullscreen_triangle_geometry_at(0.0));
    let nearer = assets.create_geometry(fullscreen_triangle_geometry_at(0.8));
    let farther = assets.create_geometry(fullscreen_triangle_geometry_at(0.2));
    let blue = assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.0, 0.0, 1.0)));
    let red_blend = assets.create_material(
        MaterialDesc::unlit(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.5))
            .with_alpha_mode(AlphaMode::Blend),
    );
    let green_blend = assets.create_material(
        MaterialDesc::unlit(Color::from_linear_rgba(0.0, 1.0, 0.0, 0.5))
            .with_alpha_mode(AlphaMode::Blend),
    );
    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(background, blue)
        .add()
        .expect("background mesh inserts");
    scene
        .mesh(nearer, red_blend)
        .add()
        .expect("near transparent mesh inserts first");
    scene
        .mesh(farther, green_blend)
        .add()
        .expect("far transparent mesh inserts second");
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("camera-space blend meshes prepare");
    renderer.render(&scene, camera).expect("render succeeds");

    assert_all_pixels(renderer.frame_rgba8(), 4, 4, [181, 126, 126, 255]);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn headless_gpu_alpha_blends_sorted_asset_meshes_when_available() {
    if !unstable_headless_gpu_release_tests_enabled() {
        record_fail_closed_headless_gpu_lane(
            "headless_gpu_alpha_blends_sorted_asset_meshes_when_available",
            "local headless GPU alpha-blend readback is not trusted as release evidence in the default cargo-test lane",
        );
        return;
    }

    assert_eq!(
        Capabilities::for_gpu_backend(Backend::HeadlessGpu).alpha_pipeline,
        AlphaPipelineStatus::LinearSourceOver
    );

    let assets = Assets::new();
    let background = assets.create_geometry(fullscreen_triangle_geometry_at(0.0));
    let near = assets.create_geometry(fullscreen_triangle_geometry_at(0.8));
    let far = assets.create_geometry(fullscreen_triangle_geometry_at(0.2));
    let blue = assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.0, 0.0, 1.0)));
    let red_blend = assets.create_material(
        MaterialDesc::unlit(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.5))
            .with_alpha_mode(AlphaMode::Blend),
    );
    let green_blend = assets.create_material(
        MaterialDesc::unlit(Color::from_linear_rgba(0.0, 1.0, 0.0, 0.5))
            .with_alpha_mode(AlphaMode::Blend),
    );
    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(background, blue)
        .add()
        .expect("background mesh inserts");
    scene
        .mesh(near, red_blend)
        .add()
        .expect("near transparent mesh inserts first");
    scene
        .mesh(far, green_blend)
        .add()
        .expect("far transparent mesh inserts second");

    if let Ok(mut renderer) = Renderer::headless_gpu(4, 4) {
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("blend meshes prepare for gpu");
        renderer
            .render(&scene, camera)
            .expect("gpu blend mesh renders");

        assert_all_pixels_close(renderer.frame_rgba8(), 4, 4, [181, 126, 126, 255], 8);
    }
}

#[test]
fn prepare_with_assets_renders_line_material_as_screen_space_stroke() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::line(
        Vec3::new(-2.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
    ));
    let material = assets.create_material(MaterialDesc::line(Color::WHITE, 1.0));
    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(geometry, material)
        .add()
        .expect("line mesh inserts");
    let mut renderer = Renderer::headless(8, 8).expect("headless renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("line material prepares");
    renderer.render(&scene, camera).expect("line renders");

    assert_eq!(pixel_at(renderer.frame_rgba8(), 8, 4, 3), [80, 80, 80, 255]);
    assert_eq!(pixel_at(renderer.frame_rgba8(), 8, 4, 2), [80, 80, 80, 255]);
    assert_eq!(pixel_at(renderer.frame_rgba8(), 8, 4, 4), [80, 80, 80, 255]);
}

#[test]
fn prepare_with_assets_renders_wireframe_material_triangle_edges() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(flat_square_geometry());
    let material = assets.create_material(MaterialDesc::wireframe(Color::WHITE, 1.0));
    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(geometry, material)
        .add()
        .expect("wireframe mesh inserts");
    let mut renderer = Renderer::headless(16, 16).expect("headless renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("wireframe material prepares");
    renderer.render(&scene, camera).expect("wireframe renders");

    assert_eq!(
        pixel_at(renderer.frame_rgba8(), 16, 8, 13),
        [80, 80, 80, 255]
    );
    assert_eq!(
        pixel_at(renderer.frame_rgba8(), 16, 7, 7),
        [80, 80, 80, 255]
    );
}

#[test]
fn prepare_with_assets_renders_edge_material_without_coplanar_internal_edges() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(flat_square_geometry());
    let material = assets.create_material(MaterialDesc::edge(Color::WHITE, 1.0));
    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(geometry, material)
        .add()
        .expect("edge mesh inserts");
    let mut renderer = Renderer::headless(16, 16).expect("headless renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("edge material prepares");
    renderer
        .render(&scene, camera)
        .expect("edge material renders");

    assert_eq!(
        pixel_at(renderer.frame_rgba8(), 16, 8, 13),
        [80, 80, 80, 255]
    );
    assert_eq!(pixel_at(renderer.frame_rgba8(), 16, 7, 7), [0, 0, 0, 255]);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn headless_gpu_renders_technical_material_primitives_when_available() {
    if !unstable_headless_gpu_release_tests_enabled() {
        record_fail_closed_headless_gpu_lane(
            "headless_gpu_renders_technical_material_primitives_when_available",
            "local headless GPU technical-material rasterization is not trusted as release evidence in the default cargo-test lane",
        );
        return;
    }

    if let Ok(mut renderer) = Renderer::headless_gpu(16, 16) {
        let assets = Assets::new();
        let line_geometry = assets.create_geometry(GeometryDesc::line(
            Vec3::new(-2.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ));
        let line_material = assets.create_material(MaterialDesc::line(Color::WHITE, 2.0));
        let (mut scene, camera) = scene_with_camera();
        scene
            .mesh(line_geometry, line_material)
            .add()
            .expect("line mesh inserts");

        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("line material prepares for gpu");
        renderer.render(&scene, camera).expect("gpu line renders");
        assert!(
            count_pixels_close(renderer.frame_rgba8(), [206, 206, 206, 255], 2) >= 4,
            "gpu line material should produce multiple PBR-neutral white stroke pixels"
        );

        let assets = Assets::new();
        let wire_geometry = assets.create_geometry(flat_square_geometry());
        let wire_material = assets.create_material(MaterialDesc::wireframe(Color::WHITE, 2.0));
        let (mut scene, camera) = scene_with_camera();
        scene
            .mesh(wire_geometry, wire_material)
            .add()
            .expect("wireframe mesh inserts");

        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("wireframe material prepares for gpu");
        renderer
            .render(&scene, camera)
            .expect("gpu wireframe renders");
        assert!(
            count_pixels_close(renderer.frame_rgba8(), [206, 206, 206, 255], 2) >= 8,
            "gpu wireframe material should produce multiple PBR-neutral white edge pixels"
        );

        let assets = Assets::new();
        let edge_geometry = assets.create_geometry(flat_square_geometry());
        let edge_material = assets.create_material(MaterialDesc::edge(Color::WHITE, 2.0));
        let (mut scene, camera) = scene_with_camera();
        scene
            .mesh(edge_geometry, edge_material)
            .add()
            .expect("edge mesh inserts");

        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("edge material prepares for gpu");
        renderer
            .render(&scene, camera)
            .expect("gpu edge material renders");
        assert!(
            count_pixels_close(renderer.frame_rgba8(), [206, 206, 206, 255], 2) >= 4,
            "gpu edge material should produce multiple PBR-neutral white outer-edge pixels"
        );
        assert_eq!(pixel_at(renderer.frame_rgba8(), 16, 7, 7), [0, 0, 0, 255]);
    }
}

/// Plan line 778 commit 4: when two prepared materials share
/// `(sampler, format, dimensions)` for every populated role, the renderer
/// allocates one shared `texture_2d_array<f32>` per role and serves both
/// draws from a single material bind group via dynamic-offset uniforms.
/// This drops the observable `RendererStats::material_bind_groups` from
/// `material_count + 1` (per-material fall-back: synthetic + 2) to `1`
/// (batched).
#[test]
#[cfg(not(target_arch = "wasm32"))]
fn texture_array_batching_collapses_to_single_bind() {
    let Ok(mut renderer) = Renderer::headless_gpu(8, 8) else {
        return;
    };

    let assets = Assets::new();
    let albedo_red = pollster::block_on(assets.load_texture(
        inline_pixel_png_uri([200, 60, 60, 255]),
        TextureColorSpace::Srgb,
    ))
    .expect("inline red texture loads");
    let albedo_blue = pollster::block_on(assets.load_texture(
        inline_pixel_png_uri([60, 60, 200, 255]),
        TextureColorSpace::Srgb,
    ))
    .expect("inline blue texture loads");
    let geometry_a = assets.create_geometry(flat_square_geometry());
    let geometry_b = assets.create_geometry(flat_square_geometry());
    let material_red = assets
        .create_material(MaterialDesc::unlit(Color::WHITE).with_base_color_texture(albedo_red));
    let material_blue = assets
        .create_material(MaterialDesc::unlit(Color::WHITE).with_base_color_texture(albedo_blue));

    let (mut scene, _camera) = scene_with_camera();
    scene
        .mesh(geometry_a, material_red)
        .add()
        .expect("red mesh inserts");
    scene
        .mesh(geometry_b, material_blue)
        .transform(Transform::at(Vec3::new(1.5, 0.0, 0.0)))
        .add()
        .expect("blue mesh inserts");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("two-material scene prepares for headless GPU");

    let stats = renderer.stats();
    assert!(
        stats.material_batch_layers >= 2,
        "two compatible materials should yield a 2+ layer texture_2d_array, \
         got material_batch_layers={}",
        stats.material_batch_layers
    );
    assert_eq!(
        stats.material_bind_groups, 1,
        "compatible materials must collapse into a single shared material bind group, \
         got material_bind_groups={}",
        stats.material_bind_groups
    );
}

/// Plan line 778 commit 4 negative path: when materials disagree on
/// `(sampler, format, dimensions)`, the batched path is unavailable and
/// the renderer falls back to one bind group per material (plus the
/// synthetic fallback at index 0). The stat reports
/// `material_count + 1` distinct bind groups, proving the fall-back path
/// is still wired correctly.
#[test]
#[cfg(not(target_arch = "wasm32"))]
fn texture_array_batching_falls_back_when_dimensions_mismatch() {
    let Ok(mut renderer) = Renderer::headless_gpu(8, 8) else {
        return;
    };

    let assets = Assets::new();
    // Red is 1x1, blue is 2x2 — dimension mismatch blocks array batching.
    let albedo_red = pollster::block_on(assets.load_texture(
        inline_pixel_png_uri([200, 60, 60, 255]),
        TextureColorSpace::Srgb,
    ))
    .expect("inline 1x1 red texture loads");
    let albedo_blue_2x2 = pollster::block_on(assets.load_texture(
        inline_2x2_png_uri([60, 60, 200, 255]),
        TextureColorSpace::Srgb,
    ))
    .expect("inline 2x2 blue texture loads");
    let geometry_a = assets.create_geometry(flat_square_geometry());
    let geometry_b = assets.create_geometry(flat_square_geometry());
    let material_red = assets
        .create_material(MaterialDesc::unlit(Color::WHITE).with_base_color_texture(albedo_red));
    let material_blue = assets.create_material(
        MaterialDesc::unlit(Color::WHITE).with_base_color_texture(albedo_blue_2x2),
    );

    let (mut scene, _camera) = scene_with_camera();
    scene
        .mesh(geometry_a, material_red)
        .add()
        .expect("red mesh inserts");
    scene
        .mesh(geometry_b, material_blue)
        .transform(Transform::at(Vec3::new(1.5, 0.0, 0.0)))
        .add()
        .expect("blue mesh inserts");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("incompatible-material scene prepares for headless GPU");

    let stats = renderer.stats();
    assert_eq!(
        stats.material_batch_layers, 0,
        "dimension mismatch must mark the batch plan unbatchable, \
         got material_batch_layers={}",
        stats.material_batch_layers
    );
    assert_eq!(
        stats.material_bind_groups, 3,
        "per-material fall-back must allocate one bind group per material slot \
         plus the synthetic fallback at index 0, got material_bind_groups={}",
        stats.material_bind_groups
    );
}

/// Regression: when one material in a batch carries a base-color texture
/// of N×N pixels and another material has no base-color texture at all, the
/// batched `texture_2d_array<f32>` layer for the textureless material must
/// be filled with a properly-sized fallback so the upload covers the
/// template's pixel count. The previous bug uploaded a 1×1 fallback into
/// a N×N slot, producing
/// `Copy at offset 0 for N*N*4 bytes would end up overrunning the bounds
/// of the Source buffer of size 4`.
#[test]
#[cfg(not(target_arch = "wasm32"))]
fn texture_array_batching_handles_materials_with_and_without_textures() {
    let Ok(mut renderer) = Renderer::headless_gpu(8, 8) else {
        return;
    };

    let assets = Assets::new();
    let albedo = pollster::block_on(assets.load_texture(
        inline_2x2_png_uri([200, 60, 60, 255]),
        TextureColorSpace::Srgb,
    ))
    .expect("inline textured albedo loads");
    let geometry_a = assets.create_geometry(flat_square_geometry());
    let geometry_b = assets.create_geometry(flat_square_geometry());
    let material_textured = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8)
            .with_base_color_texture(albedo),
    );
    let material_plain = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(60, 200, 60),
        0.0,
        0.8,
    ));

    let (mut scene, _camera) = scene_with_camera();
    scene
        .mesh(geometry_a, material_textured)
        .add()
        .expect("textured mesh inserts");
    scene
        .mesh(geometry_b, material_plain)
        .transform(Transform::at(Vec3::new(1.5, 0.0, 0.0)))
        .add()
        .expect("plain mesh inserts");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("mixed-texture scene prepares without crashing the batched array upload");

    // Confirm we exercised the batched path; otherwise the test would
    // silently pass on per-material fallback and not regress the original
    // crash from `target/gate-artifacts/m8-real-asset` (256×256 WaterBottle
    // base color + 1×1 floor fallback).
    let stats = renderer.stats();
    assert!(
        stats.material_batch_layers >= 2,
        "batched path must engage when materials share (sampler, format, dimensions); \
         got material_batch_layers={}",
        stats.material_batch_layers
    );
}

/// Regression: a scene with a shadow-casting directional light + multiple
/// renderable meshes used to crash with
/// `TextureUses(DEPTH_STENCIL_WRITE) is an exclusive usage and cannot be
/// used with any other usages within the usage scope`. The shadow caster
/// pass and the unlit pass were both attached to a command encoder that
/// referenced the shadow_map as both a depth-stencil write target and as
/// an output-bind-group resource within the same usage scope.
#[test]
#[cfg(not(target_arch = "wasm32"))]
fn shadow_casting_light_with_multiple_meshes_renders_without_validation_error() {
    let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
        return;
    };

    let assets = Assets::new();
    let geometry_a = assets.create_geometry(flat_square_geometry());
    let geometry_b = assets.create_geometry(flat_square_geometry());
    let material_a =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.5));
    let material_b = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(180, 80, 80),
        0.0,
        0.5,
    ));

    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(geometry_a, material_a)
        .add()
        .expect("first mesh inserts");
    scene
        .mesh(geometry_b, material_b)
        .transform(Transform::at(Vec3::new(0.0, 0.0, -0.5)))
        .add()
        .expect("second mesh inserts");
    scene
        .directional_light(
            scena::DirectionalLight::default()
                .with_color(Color::WHITE)
                .with_illuminance_lux(50_000.0)
                .with_shadows(true),
        )
        .transform(Transform::default().rotate_x_deg(-45.0).rotate_y_deg(30.0))
        .add()
        .expect("shadow-casting key light inserts");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene with shadow-casting light prepares for GPU");
    renderer
        .render(&scene, camera)
        .expect("scene with shadow-casting light renders without validation error");
}

#[cfg(not(target_arch = "wasm32"))]
fn inline_pixel_png_uri(pixel: [u8; 4]) -> String {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(std::io::Cursor::new(&mut bytes), 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header writes");
        writer.write_image_data(&pixel).expect("PNG payload writes");
    }
    format!(
        "data:image/png;base64,{}",
        <base64::engine::general_purpose::GeneralPurpose as base64::Engine>::encode(
            &base64::engine::general_purpose::STANDARD,
            bytes,
        )
    )
}

#[cfg(not(target_arch = "wasm32"))]
fn inline_2x2_png_uri(pixel: [u8; 4]) -> String {
    let mut bytes = Vec::new();
    let pixels: [u8; 16] = [
        pixel[0], pixel[1], pixel[2], pixel[3], pixel[0], pixel[1], pixel[2], pixel[3], pixel[0],
        pixel[1], pixel[2], pixel[3], pixel[0], pixel[1], pixel[2], pixel[3],
    ];
    {
        let mut encoder = png::Encoder::new(std::io::Cursor::new(&mut bytes), 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header writes");
        writer
            .write_image_data(&pixels)
            .expect("PNG payload writes");
    }
    format!(
        "data:image/png;base64,{}",
        <base64::engine::general_purpose::GeneralPurpose as base64::Engine>::encode(
            &base64::engine::general_purpose::STANDARD,
            bytes,
        )
    )
}

#[test]
fn prepare_without_assets_rejects_asset_backed_mesh_nodes() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let (mut scene, _camera) = scene_with_camera();
    let mesh_node = scene
        .mesh(geometry, material)
        .add()
        .expect("mesh node inserts");
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");

    assert!(matches!(
        renderer.prepare(&mut scene),
        Err(PrepareError::AssetsRequired { node }) if node == mesh_node
    ));
}

#[test]
fn prepare_with_assets_rejects_unsupported_mesh_inputs_structurally() {
    let assets = Assets::new();
    let valid_geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let valid_material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let line_geometry =
        assets.create_geometry(GeometryDesc::line(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)));
    let line_material = assets.create_material(MaterialDesc::line(Color::WHITE, 1.0));
    let mask_material = assets.create_material(
        MaterialDesc::unlit(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.25))
            .with_alpha_mode(AlphaMode::Mask { cutoff: 0.5 }),
    );

    let (node, error) = prepare_mesh_error(&assets, GeometryHandle::default(), valid_material);
    assert!(matches!(
        error,
        PrepareError::GeometryNotFound { node: error_node, geometry }
            if error_node == node && geometry == GeometryHandle::default()
    ));

    let (node, error) = prepare_mesh_error(&assets, valid_geometry, MaterialHandle::default());
    assert!(matches!(
        error,
        PrepareError::MaterialNotFound { node: error_node, material }
            if error_node == node && material == MaterialHandle::default()
    ));

    let (node, error) = prepare_mesh_error(&assets, line_geometry, valid_material);
    assert!(matches!(
        error,
        PrepareError::UnsupportedGeometryTopology { node: error_node, topology: GeometryTopology::Lines }
            if error_node == node
    ));

    let (node, error) = prepare_mesh_error(&assets, valid_geometry, line_material);
    assert!(matches!(
        error,
        PrepareError::UnsupportedMaterialKind { node: error_node, kind: MaterialKind::Line }
            if error_node == node
    ));

    let (mut masked_scene, camera) = scene_with_camera();
    masked_scene
        .mesh(valid_geometry, mask_material)
        .add()
        .expect("masked mesh inserts");
    let mut masked_renderer = Renderer::headless(4, 4).expect("headless renderer builds");
    masked_renderer
        .prepare_with_assets(&mut masked_scene, &assets)
        .expect("alpha mask prepares as an opaque cutoff pass");
    assert_eq!(
        masked_renderer
            .render(&masked_scene, camera)
            .expect("masked scene renders")
            .draw_calls,
        0,
        "constant alpha below cutoff should discard the prepared primitive"
    );

    let (mut scene, _camera) = scene_with_camera();
    let model_node = scene
        .model(ModelHandle::default())
        .add()
        .expect("model node inserts");
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");
    assert!(matches!(
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect_err("model nodes are not part of M1 forward opaque prepare"),
        PrepareError::UnsupportedModelNode { node } if node == model_node
    ));
}

fn prepare_mesh_error(
    assets: &Assets,
    geometry: GeometryHandle,
    material: MaterialHandle,
) -> (scena::NodeKey, PrepareError) {
    let (mut scene, _camera) = scene_with_camera();
    let node = scene
        .mesh(geometry, material)
        .add()
        .expect("mesh node inserts");
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");
    let error = renderer
        .prepare_with_assets(&mut scene, assets)
        .expect_err("mesh input should be rejected structurally");
    (node, error)
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn headless_gpu_output_stage_applies_pbr_neutral_srgb_for_pinned_white_fixture() {
    assert_eq!(
        Capabilities::for_gpu_backend(Backend::HeadlessGpu).output_stage,
        OutputStageStatus::PbrNeutralSrgb
    );

    if let Ok(mut renderer) = Renderer::headless_gpu(4, 4) {
        let (mut scene, camera) = scene_with_fullscreen_triangle(Color::WHITE);
        assert_eq!(
            renderer.capabilities().output_stage,
            OutputStageStatus::PbrNeutralSrgb
        );
        assert_eq!(
            renderer.capabilities().alpha_pipeline,
            AlphaPipelineStatus::LinearSourceOver
        );

        renderer.prepare(&mut scene).expect("gpu scene prepares");
        renderer.render(&scene, camera).expect("gpu scene renders");

        assert_eq!(
            center_pixel(renderer.frame_rgba8(), 4, 4),
            [206, 206, 206, 255]
        );

        renderer.set_exposure_ev(2.0);
        renderer
            .render_active(&scene)
            .expect("gpu exposure update renders without reprepare");
        assert_eq!(
            center_pixel(renderer.frame_rgba8(), 4, 4),
            [253, 253, 253, 255]
        );
    }
}

#[test]
fn assets_create_material_stores_descriptor_by_typed_handle() {
    let assets = Assets::new();
    let material = MaterialDesc::unlit(Color::from_linear_rgb(0.25, 0.5, 0.75));

    let handle = assets.create_material(material.clone());
    let retrieved: Option<MaterialDesc> = assets.material(handle);

    assert_eq!(retrieved, Some(material));
}

#[test]
fn unlit_material_descriptor_sets_non_pbr_defaults() {
    const MATERIAL: MaterialDesc = MaterialDesc::unlit(Color::from_linear_rgb(0.25, 0.5, 0.75));

    assert_eq!(MATERIAL.kind(), MaterialKind::Unlit);
    assert_eq!(
        MATERIAL.base_color(),
        Color::from_linear_rgb(0.25, 0.5, 0.75)
    );
    assert_eq!(MATERIAL.metallic_factor(), 0.0);
    assert_eq!(MATERIAL.roughness_factor(), 1.0);
    assert_eq!(MATERIAL.alpha_mode(), AlphaMode::Opaque);
}

#[test]
fn pbr_material_factors_are_const_and_sanitized() {
    const CLAMPED: MaterialDesc = MaterialDesc::pbr_metallic_roughness(Color::WHITE, 2.0, -1.0);

    assert_eq!(CLAMPED.kind(), MaterialKind::PbrMetallicRoughness);
    assert_eq!(CLAMPED.metallic_factor(), 1.0);
    assert_eq!(CLAMPED.roughness_factor(), 0.0);

    let sanitized_nan = MaterialDesc::pbr_metallic_roughness(Color::WHITE, f32::NAN, f32::NAN);
    assert_eq!(sanitized_nan.metallic_factor(), 0.0);
    assert_eq!(sanitized_nan.roughness_factor(), 1.0);
    assert!(!sanitized_nan.metallic_factor().is_nan());
    assert!(!sanitized_nan.roughness_factor().is_nan());
}

#[test]
fn material_texture_slot_helpers_store_handles_without_color_space_duplication() {
    let assets = Assets::new();
    let albedo =
        pollster::block_on(assets.load_texture("paint_basecolor.png", TextureColorSpace::Srgb))
            .expect("albedo request is recorded");
    let normal =
        pollster::block_on(assets.load_texture("paint_normal.png", TextureColorSpace::Linear))
            .expect("normal request is recorded");
    let metallic_roughness = pollster::block_on(
        assets.load_texture("paint_metallic_roughness.png", TextureColorSpace::Linear),
    )
    .expect("metallic roughness request is recorded");
    let occlusion =
        pollster::block_on(assets.load_texture("paint_occlusion.png", TextureColorSpace::Linear))
            .expect("occlusion request is recorded");
    let emissive =
        pollster::block_on(assets.load_texture("paint_emissive.png", TextureColorSpace::Srgb))
            .expect("emissive request is recorded");

    let material = MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.2, 0.8)
        .with_base_color_texture(albedo)
        .with_normal_texture(normal)
        .with_metallic_roughness_texture(metallic_roughness)
        .with_occlusion_texture(occlusion)
        .with_emissive_texture(emissive);

    assert_eq!(material.base_color_texture(), Some(albedo));
    assert_eq!(material.normal_texture(), Some(normal));
    assert_eq!(
        material.metallic_roughness_texture(),
        Some(metallic_roughness)
    );
    assert_eq!(material.occlusion_texture(), Some(occlusion));
    assert_eq!(material.emissive_texture(), Some(emissive));
}

#[test]
fn alpha_and_emissive_helpers_sanitize_descriptor_values() {
    const MASKED: MaterialDesc = MaterialDesc::unlit(Color::WHITE)
        .with_alpha_mode(AlphaMode::Mask { cutoff: 0.4 })
        .with_emissive(Color::from_linear_rgb(0.1, 0.2, 0.3))
        .with_emissive_strength(2.5)
        .with_double_sided(true);

    assert_eq!(MASKED.alpha_mode(), AlphaMode::Mask { cutoff: 0.4 });
    assert_eq!(MASKED.emissive(), Color::from_linear_rgb(0.1, 0.2, 0.3));
    assert_eq!(MASKED.emissive_strength(), 2.5);
    assert!(MASKED.double_sided());

    const HIGH_CUTOFF: MaterialDesc =
        MaterialDesc::unlit(Color::WHITE).with_alpha_mode(AlphaMode::Mask { cutoff: 2.0 });
    const NAN_CUTOFF: MaterialDesc =
        MaterialDesc::unlit(Color::WHITE).with_alpha_mode(AlphaMode::Mask { cutoff: f32::NAN });
    const NEGATIVE_EMISSIVE: MaterialDesc =
        MaterialDesc::unlit(Color::WHITE).with_emissive_strength(-2.0);
    const NAN_EMISSIVE: MaterialDesc =
        MaterialDesc::unlit(Color::WHITE).with_emissive_strength(f32::NAN);

    assert_eq!(HIGH_CUTOFF.alpha_mode(), AlphaMode::Mask { cutoff: 1.0 });
    assert_eq!(NAN_CUTOFF.alpha_mode(), AlphaMode::Mask { cutoff: 0.5 });
    assert_eq!(NEGATIVE_EMISSIVE.emissive_strength(), 0.0);
    assert_eq!(NAN_EMISSIVE.emissive_strength(), 1.0);

    let transparent = MASKED.with_alpha_mode(AlphaMode::Blend);
    assert_eq!(transparent.alpha_mode(), AlphaMode::Blend);
}

#[test]
fn technical_material_descriptors_capture_line_wireframe_and_edge_contracts() {
    const LINE: MaterialDesc = MaterialDesc::line(Color::WHITE, 0.0);
    const WIREFRAME: MaterialDesc =
        MaterialDesc::wireframe(Color::from_linear_rgb(0.1, 0.2, 0.3), f32::NAN)
            .with_stroke_width_px(f32::INFINITY);
    const EDGE: MaterialDesc = MaterialDesc::edge(Color::from_linear_rgb(0.8, 0.7, 0.6), 2.5)
        .with_edge_angle_threshold_degrees(400.0);
    const NEGATIVE_EDGE: MaterialDesc =
        MaterialDesc::edge(Color::WHITE, -4.0).with_edge_angle_threshold_degrees(-20.0);
    const NAN_EDGE: MaterialDesc =
        MaterialDesc::edge(Color::WHITE, 3.0).with_edge_angle_threshold_degrees(f32::NAN);
    const NON_STROKE: MaterialDesc = MaterialDesc::unlit(Color::WHITE).with_stroke_width_px(5.0);
    const NON_EDGE: MaterialDesc =
        MaterialDesc::line(Color::WHITE, 2.0).with_edge_angle_threshold_degrees(45.0);

    assert_eq!(LINE.kind(), MaterialKind::Line);
    assert_eq!(LINE.base_color(), Color::WHITE);
    assert_eq!(LINE.stroke_width_px(), Some(DEFAULT_STROKE_WIDTH_PX));
    assert_eq!(LINE.edge_angle_threshold_degrees(), None);

    assert_eq!(WIREFRAME.kind(), MaterialKind::Wireframe);
    assert_eq!(
        WIREFRAME.base_color(),
        Color::from_linear_rgb(0.1, 0.2, 0.3)
    );
    assert_eq!(WIREFRAME.stroke_width_px(), Some(1.0));
    assert_eq!(WIREFRAME.edge_angle_threshold_degrees(), None);

    assert_eq!(EDGE.kind(), MaterialKind::Edge);
    assert_eq!(EDGE.base_color(), Color::from_linear_rgb(0.8, 0.7, 0.6));
    assert_eq!(EDGE.stroke_width_px(), Some(2.5));
    assert_eq!(EDGE.edge_angle_threshold_degrees(), Some(180.0));
    assert_eq!(NEGATIVE_EDGE.stroke_width_px(), Some(1.0));
    assert_eq!(NEGATIVE_EDGE.edge_angle_threshold_degrees(), Some(0.0));
    assert_eq!(
        NAN_EDGE.edge_angle_threshold_degrees(),
        Some(DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES)
    );
    assert_eq!(NON_STROKE.stroke_width_px(), None);
    assert_eq!(NON_EDGE.edge_angle_threshold_degrees(), None);
}

#[test]
fn assets_load_texture_records_color_space_and_deduplicates_cache_key() {
    let assets = Assets::new();

    let first =
        pollster::block_on(assets.load_texture("textures/albedo.png", TextureColorSpace::Srgb))
            .expect("texture request is recorded");
    let duplicate =
        pollster::block_on(assets.load_texture("textures/albedo.png", TextureColorSpace::Srgb))
            .expect("duplicate texture request is recorded");
    let linear =
        pollster::block_on(assets.load_texture("textures/albedo.png", TextureColorSpace::Linear))
            .expect("linear texture request is recorded");
    let first_texture: Option<TextureDesc> = assets.texture(first);

    assert_eq!(first, duplicate);
    assert_ne!(first, linear);
    assert_eq!(
        first_texture.as_ref().map(|texture| texture.path()),
        Some(&AssetPath::from("textures/albedo.png"))
    );
    assert_eq!(
        first_texture.map(|texture| texture.color_space()),
        Some(TextureColorSpace::Srgb)
    );
    assert_eq!(
        assets.texture(linear).map(|texture| texture.color_space()),
        Some(TextureColorSpace::Linear)
    );
}

fn assert_geometry_invariants(geometry: &GeometryDesc) {
    assert!(!geometry.vertices().is_empty());
    for index in geometry.indices() {
        assert!((*index as usize) < geometry.vertices().len());
    }
    for vertex in geometry.vertices() {
        assert!(geometry.bounds().contains(vertex.position));
    }
    match geometry.topology() {
        GeometryTopology::Triangles => {
            assert_eq!(geometry.indices().len() % 3, 0);
            for vertex in geometry.vertices() {
                let length = vector_length(vertex.normal);
                assert!(
                    (length - 1.0).abs() <= 1.0e-4,
                    "triangle geometry normals must be unit length, got {length}"
                );
            }
        }
        GeometryTopology::Lines => assert_eq!(geometry.indices().len() % 2, 0),
    }
}

fn vector_length(value: Vec3) -> f32 {
    (value.x * value.x + value.y * value.y + value.z * value.z).sqrt()
}

#[test]
fn builtin_geometry_generators_produce_valid_bounds_and_indices() {
    let geometries = [
        GeometryDesc::box_xyz(2.0, 4.0, 6.0),
        GeometryDesc::sphere(1.5, 12, 6),
        GeometryDesc::cylinder(1.0, 3.0, 12),
        GeometryDesc::plane(2.0, 3.0),
        GeometryDesc::line(Vec3::ZERO, Vec3::new(1.0, 2.0, 3.0)),
        GeometryDesc::polyline(&[
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
        ]),
        GeometryDesc::arrow(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0)),
        GeometryDesc::grid(10.0, 10),
        GeometryDesc::axes(2.0),
    ];

    for geometry in geometries {
        assert_geometry_invariants(&geometry);
    }
    assert_eq!(
        GeometryDesc::box_xyz(2.0, 4.0, 6.0).bounds(),
        Aabb::new(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(1.0, 2.0, 3.0))
    );
}

#[test]
fn geometry_construction_rejects_invalid_manual_buffers() {
    assert!(Aabb::from_vertices(&[]).is_none());
    assert!(GeometryDesc::try_new(GeometryTopology::Triangles, Vec::new(), Vec::new()).is_err());
    assert!(
        GeometryDesc::try_new(
            GeometryTopology::Triangles,
            GeometryDesc::plane(1.0, 1.0).vertices().to_vec(),
            vec![0, 1]
        )
        .is_err()
    );
    assert!(
        GeometryDesc::try_new(
            GeometryTopology::Lines,
            GeometryDesc::line(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0))
                .vertices()
                .to_vec(),
            vec![0, 2]
        )
        .is_err()
    );
}

#[test]
fn assets_create_geometry_stores_descriptor_by_typed_handle() {
    let assets = Assets::new();
    let geometry = GeometryDesc::plane(2.0, 2.0);

    let handle = assets.create_geometry(geometry.clone());

    assert_eq!(assets.geometry(handle), Some(geometry));
}

#[test]
fn scene_mesh_builder_inserts_typed_mesh_node() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::plane(1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();

    let node = scene
        .mesh(geometry, material)
        .transform(Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            ..Transform::default()
        })
        .add()
        .expect("mesh node inserts under root");

    let node = scene.node(node).expect("mesh node exists");
    assert_eq!(node.transform().translation, Vec3::new(1.0, 2.0, 3.0));
    match node.kind() {
        NodeKind::Mesh(mesh) => {
            assert_eq!(mesh.geometry(), geometry);
            assert_eq!(mesh.material(), material);
        }
        other => panic!("expected mesh node, got {other:?}"),
    }
}

#[test]
fn scene_model_builder_inserts_typed_model_node_under_parent() {
    let mut scene = Scene::new();
    let parent = scene
        .add_empty(scene.root(), Transform::default())
        .expect("parent inserts");
    // Model loading lands with glTF/model assets; this sentinel only proves the
    // scene-side typed-handle insertion path.
    let model = ModelHandle::default();

    let node = scene
        .model(model)
        .parent(parent)
        .add()
        .expect("model node inserts under parent");

    let node = scene.node(node).expect("model node exists");
    assert_eq!(node.parent(), Some(parent));
    match node.kind() {
        NodeKind::Model(model_node) => assert_eq!(model_node.model(), model),
        other => panic!("expected model node, got {other:?}"),
    }
}
