#![cfg(feature = "inspection")]

use scena::{
    Aabb, Assets, Backend, CAPTURE_SCHEMA_V1, CaptureDescriptor, CaptureError, CaptureOptions,
    CapturePayloadKind, CaptureRevisions, Color, FramingOptions, GeometryDesc, MaterialDesc,
    NodeKey, PerspectiveCamera, Renderer, Scene, Transform, Vec3, capture_rgba8,
    headless_gltf_viewer,
};
#[cfg(feature = "scene-host")]
use scena::{SceneHostCore, SceneInspectionReportV1};

#[test]
fn capture_descriptor_schema_round_trips_and_binds_revisions_to_inspection() {
    let (assets, scene, renderer) = rendered_box_scene(64, 64);

    let capture =
        capture_rgba8(&scene, &renderer, CaptureOptions::default()).expect("capture succeeds");
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();

    assert_eq!(capture.descriptor.schema, CAPTURE_SCHEMA_V1);
    assert_eq!(capture.descriptor.width, 64);
    assert_eq!(capture.descriptor.height, 64);
    assert_eq!(capture.descriptor.pixel_format, "rgba8");
    assert_eq!(capture.descriptor.payload.kind, CapturePayloadKind::Rgba8);
    assert_eq!(capture.descriptor.payload.byte_length, capture.rgba8.len());
    assert_eq!(
        capture.descriptor.revisions,
        CaptureRevisions {
            structure: inspection.revisions.structure,
            transform: inspection.revisions.transform,
            interaction: inspection.revisions.interaction,
        }
    );
    assert!(capture.descriptor.camera.active);
    assert!(capture.descriptor.camera.world_transform.is_some());
    assert!(matches!(
        capture
            .descriptor
            .camera
            .projection
            .as_ref()
            .map(|projection| projection.kind()),
        Some("perspective")
    ));
    assert_eq!(capture.descriptor.backend, Backend::Headless);
    assert_eq!(capture.descriptor.viewport.device_pixel_ratio, 1.0);
    assert!(capture.descriptor.pixels.nonblack > 0);
    assert!(capture.descriptor.pixels.bbox.is_some());
    assert_eq!(
        capture.descriptor.pixels.fnv1a64,
        scena::fnv1a64_hex(capture.rgba8.as_slice())
    );

    let schema_json = capture.descriptor.to_schema_json();
    assert_eq!(schema_json["schema"], CAPTURE_SCHEMA_V1);
    assert_eq!(schema_json["payload"]["kind"], "rgba8");
    assert_eq!(
        schema_json["revisions"]["structure"],
        inspection.revisions.structure
    );
    let decoded: CaptureDescriptor =
        serde_json::from_value(schema_json).expect("capture descriptor deserializes");
    assert_eq!(decoded, capture.descriptor);
}

#[test]
fn capture_descriptor_records_auto_frame_projection_metadata() {
    let (assets, mut scene, mut renderer) = box_scene_with_camera(96, 72);
    let camera = scene.active_camera().expect("active camera exists");
    let bounds = Aabb::new(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5));
    scene
        .frame_bounds(
            camera,
            bounds,
            FramingOptions::new()
                .viewport(96, 72)
                .fill(0.60)
                .margin_px(2.0),
        )
        .expect("camera frames bounds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    renderer.render_active(&scene).expect("scene renders");

    let capture = capture_rgba8(
        &scene,
        &renderer,
        CaptureOptions::default()
            .with_device_pixel_ratio(2.0)
            .with_auto_frame_bounds(bounds),
    )
    .expect("capture succeeds");

    let auto_frame = capture
        .descriptor
        .auto_frame
        .expect("auto-frame metadata is captured");
    assert_eq!(capture.descriptor.viewport.logical_width, 48.0);
    assert_eq!(capture.descriptor.viewport.logical_height, 36.0);
    assert!(auto_frame.inside_viewport);
    assert!(auto_frame.centered);
    assert!(auto_frame.fill_fraction > 0.2);
    assert!(auto_frame.projected_rect.width > 0.0);
    assert!(auto_frame.projected_rect.height > 0.0);
}

#[test]
fn headless_viewer_capture_exposes_descriptor_and_auto_frame_metadata() {
    let first = pollster::block_on(
        headless_gltf_viewer("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            .size(80, 80)
            .render(),
    )
    .expect("viewer renders");

    let capture = first.capture().expect("viewer capture succeeds");

    assert_eq!(capture.descriptor.schema, CAPTURE_SCHEMA_V1);
    assert_eq!(capture.descriptor.width, 80);
    assert_eq!(capture.descriptor.height, 80);
    assert_eq!(capture.descriptor.backend, Backend::Headless);
    assert_eq!(
        capture
            .descriptor
            .auto_frame
            .as_ref()
            .expect("viewer capture includes import auto-frame metadata")
            .proof_class,
        "viewer-level-auto-framing"
    );
    assert_eq!(capture.rgba8, first.renderer().frame_rgba8());
}

#[test]
fn cpu_headless_capture_is_deterministic_for_the_same_scene_state() {
    let first = {
        let (_assets, scene, renderer) = rendered_box_scene(48, 48);
        renderer
            .capture_rgba8(&scene, CaptureOptions::default())
            .expect("first capture")
    };
    let second = {
        let (_assets, scene, renderer) = rendered_box_scene(48, 48);
        capture_rgba8(&scene, &renderer, CaptureOptions::default()).expect("second capture")
    };

    assert_eq!(first.rgba8, second.rgba8);
    assert_eq!(first.descriptor.pixels, second.descriptor.pixels);
    assert_eq!(first.descriptor.revisions, second.descriptor.revisions);
}

#[test]
fn capture_fails_closed_when_scene_mutates_after_render() {
    let (_assets, mut scene, renderer, mesh) = rendered_box_scene_with_mesh(48, 48);
    let rendered = capture_rgba8(&scene, &renderer, CaptureOptions::default())
        .expect("initial capture succeeds")
        .descriptor
        .revisions;

    scene
        .set_transform(mesh, Transform::at(Vec3::new(0.25, 0.0, 0.0)))
        .expect("mesh transform updates");

    let error = capture_rgba8(&scene, &renderer, CaptureOptions::default())
        .expect_err("capture must reject stale framebuffer metadata");
    assert!(matches!(
        error,
        CaptureError::StaleRender { rendered: stale, current }
            if stale == rendered
                && current.structure == rendered.structure
                && current.transform == rendered.transform + 1
                && current.interaction == rendered.interaction
    ));
}

#[test]
fn capture_requires_a_rendered_frame() {
    let (_assets, scene, renderer) = box_scene_with_camera(48, 48);

    let error = capture_rgba8(&scene, &renderer, CaptureOptions::default())
        .expect_err("capture before render must fail");

    assert!(matches!(error, CaptureError::NoRenderedFrame));
}

#[test]
fn capture_fails_closed_when_active_camera_changes_after_render() {
    let (assets, mut scene, mut renderer, _mesh) = box_scene_with_camera_and_mesh(48, 48);
    let rendered_camera = scene.active_camera().expect("default camera exists");
    let second_camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(1.0, 1.0, 4.0)),
        )
        .expect("second camera inserts");
    scene
        .set_active_camera(rendered_camera)
        .expect("first camera is active");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    renderer.render_active(&scene).expect("scene renders");
    let rendered = capture_rgba8(&scene, &renderer, CaptureOptions::default())
        .expect("initial capture succeeds")
        .descriptor
        .revisions;

    scene
        .set_active_camera(second_camera)
        .expect("second camera becomes active");

    let error = capture_rgba8(&scene, &renderer, CaptureOptions::default())
        .expect_err("capture must reject active-camera drift");
    assert!(matches!(
        error,
        CaptureError::StaleRender { rendered: stale, current }
            if stale == rendered && current == rendered
    ));
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_host_capture_uses_rendered_state_revisions_and_pixels() {
    let mut host = SceneHostCore::headless(64, 64).expect("host builds");
    let root = host.root_handle();
    let frame = host
        .add_empty(
            Some(root),
            Transform::at(Vec3::new(0.0, 0.0, 0.0)),
            Some("capture-frame"),
        )
        .expect("frame inserts");
    let import = pollster::block_on(host.instantiate_url_under(
        frame,
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
    ))
    .expect("asset instantiates");
    let mesh = host
        .node_handle(import, "ColoredTriangle")
        .expect("mesh handle resolves");

    host.frame_node(mesh).expect("host frames mesh");
    host.prepare().expect("host prepares");
    host.render().expect("host renders");
    let capture = host.capture().expect("host capture succeeds");
    let inspection: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");

    assert_eq!(capture.descriptor.schema, CAPTURE_SCHEMA_V1);
    assert_eq!(capture.descriptor.width, 64);
    assert_eq!(capture.descriptor.height, 64);
    assert_eq!(
        capture.descriptor.revisions,
        CaptureRevisions {
            structure: inspection.revisions.structure,
            transform: inspection.revisions.transform,
            interaction: inspection.revisions.interaction,
        }
    );
    assert_eq!(capture.descriptor.backend, host.backend());
    assert!(capture.descriptor.pixels.nonblack > 0);
    assert_eq!(capture.rgba8.len(), 64 * 64 * 4);
    assert_eq!(
        capture.descriptor.pixels.fnv1a64,
        scena::fnv1a64_hex(capture.rgba8.as_slice())
    );
}

fn rendered_box_scene(width: u32, height: u32) -> (Assets, Scene, Renderer) {
    let (assets, scene, renderer, _mesh) = rendered_box_scene_with_mesh(width, height);
    (assets, scene, renderer)
}

fn rendered_box_scene_with_mesh(width: u32, height: u32) -> (Assets, Scene, Renderer, NodeKey) {
    let (assets, mut scene, mut renderer, mesh) = box_scene_with_camera_and_mesh(width, height);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    renderer.render_active(&scene).expect("scene renders");
    (assets, scene, renderer, mesh)
}

fn box_scene_with_camera(width: u32, height: u32) -> (Assets, Scene, Renderer) {
    let (assets, scene, renderer, _mesh) = box_scene_with_camera_and_mesh(width, height);
    (assets, scene, renderer)
}

fn box_scene_with_camera_and_mesh(width: u32, height: u32) -> (Assets, Scene, Renderer, NodeKey) {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("default camera inserts");
    let mesh = scene
        .mesh(geometry, material)
        .add()
        .expect("box mesh inserts");
    let renderer = Renderer::headless(width, height).expect("headless renderer builds");
    (assets, scene, renderer, mesh)
}
