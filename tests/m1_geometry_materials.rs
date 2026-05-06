use scena::{
    Aabb, AlphaMode, AlphaPipelineStatus, AssetPath, Assets, Color,
    DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES, DEFAULT_STROKE_WIDTH_PX, EnvironmentHandle, GeometryDesc,
    GeometryHandle, GeometryTopology, MaterialDesc, MaterialHandle, MaterialKind, ModelHandle,
    NodeKind, OutputStageStatus, PerspectiveCamera, PrepareError, Primitive, Renderer, Scene,
    SceneAsset, TextureColorSpace, TextureDesc, TextureHandle, Tonemapper, Transform, Vec3, Vertex,
};

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

fn assert_all_pixels(frame: &[u8], width: u32, height: u32, expected: [u8; 4]) {
    assert_eq!(frame.len(), (width as usize) * (height as usize) * 4);
    for (index, pixel) in frame.chunks_exact(4).enumerate() {
        assert_eq!(
            pixel, expected,
            "pixel {index} should match fullscreen constant-color output"
        );
    }
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
            Transform::default(),
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
                    position: Vec3::new(-1.0, -1.0, 0.0),
                    color,
                },
                Vertex {
                    position: Vec3::new(3.0, -1.0, 0.0),
                    color,
                },
                Vertex {
                    position: Vec3::new(-1.0, 3.0, 0.0),
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
            Transform::default(),
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
            Transform::default(),
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
fn headless_output_stage_applies_aces_srgb_and_exposure_without_reprepare() {
    let (mut scene, camera) = scene_with_fullscreen_triangle(Color::WHITE);
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");

    assert_eq!(
        renderer.capabilities().output_stage,
        OutputStageStatus::AcesSrgb
    );
    assert_eq!(
        renderer.capabilities().alpha_pipeline,
        AlphaPipelineStatus::LinearSourceOver
    );
    assert_eq!(renderer.tonemapper(), Tonemapper::Aces);
    assert_eq!(renderer.exposure_ev(), 0.0);

    renderer.render(&scene, camera).expect("render succeeds");
    assert_eq!(
        center_pixel(renderer.frame_rgba8(), 4, 4),
        [206, 206, 206, 255]
    );

    renderer.set_exposure_ev(2.0);
    renderer
        .render_active(&scene)
        .expect("exposure is a steady-state update");
    assert_eq!(
        center_pixel(renderer.frame_rgba8(), 4, 4),
        [245, 245, 245, 255]
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
    assert_eq!(
        pixel_at(renderer.frame_rgba8(), 8, 2, 2),
        [206, 206, 206, 255]
    );
    assert_eq!(pixel_at(renderer.frame_rgba8(), 8, 6, 2), [0, 0, 0, 255]);
    assert_eq!(pixel_at(renderer.frame_rgba8(), 8, 2, 6), [0, 0, 0, 255]);
    assert_eq!(
        pixel_at(renderer.frame_rgba8(), 8, 6, 6),
        [206, 206, 206, 255]
    );

    assert_eq!(
        rendered_fullscreen_center_pixel(Color::BLACK),
        [0, 0, 0, 255]
    );
    assert_eq!(
        rendered_fullscreen_center_pixel(Color::WHITE),
        [206, 206, 206, 255]
    );
    assert_eq!(
        rendered_fullscreen_center_pixel(Color::from_linear_rgb(0.18, 0.18, 0.18)),
        [91, 91, 91, 255]
    );
    assert_eq!(
        rendered_fullscreen_center_pixel(Color::from_srgb(0.5, 0.5, 0.5)),
        [103, 103, 103, 255]
    );

    let color_checker = [
        (
            Color::from_linear_rgb(0.436, 0.246, 0.164),
            [153, 114, 90, 255],
        ),
        (
            Color::from_linear_rgb(0.051, 0.101, 0.411),
            [27, 59, 145, 255],
        ),
        (
            Color::from_linear_rgb(0.063, 0.239, 0.088),
            [38, 109, 57, 255],
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

    assert_all_pixels(renderer.frame_rgba8(), 4, 4, [158, 0, 159, 255]);
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

    assert_all_pixels(renderer.frame_rgba8(), 4, 4, [216, 0, 9, 255]);
}

#[test]
fn prepare_with_assets_sorts_blend_meshes_back_to_front_before_render() {
    let assets = Assets::new();
    let background = assets.create_geometry(fullscreen_triangle_geometry_at(0.0));
    let near = assets.create_geometry(fullscreen_triangle_geometry_at(0.2));
    let far = assets.create_geometry(fullscreen_triangle_geometry_at(0.8));
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
    // (0.5, 0.25, 0.25, 1.0) before ACES+sRGB output encoding.
    assert_all_pixels(renderer.frame_rgba8(), 4, 4, [163, 116, 116, 255]);
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

    assert_eq!(
        pixel_at(renderer.frame_rgba8(), 8, 4, 3),
        [206, 206, 206, 255]
    );
    assert_eq!(pixel_at(renderer.frame_rgba8(), 8, 4, 2), [0, 0, 0, 255]);
    assert_eq!(pixel_at(renderer.frame_rgba8(), 8, 4, 4), [0, 0, 0, 255]);
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

    let (node, error) = prepare_mesh_error(&assets, valid_geometry, mask_material);
    assert!(matches!(
        error,
        PrepareError::UnsupportedAlphaMode {
            node: error_node,
            alpha_mode: AlphaMode::Mask { cutoff }
        } if error_node == node && cutoff == 0.5
    ));

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
fn gpu_output_stage_divergence_is_explicit_in_capabilities() {
    if let Ok(renderer) = Renderer::headless_gpu(4, 4) {
        assert_eq!(
            renderer.capabilities().output_stage,
            OutputStageStatus::BackendPassthrough
        );
        assert_eq!(
            renderer.capabilities().alpha_pipeline,
            AlphaPipelineStatus::BackendPassthrough
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
