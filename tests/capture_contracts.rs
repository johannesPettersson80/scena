#![cfg(feature = "inspection")]

use std::io::Cursor;
use std::path::PathBuf;

use scena::{
    Aabb, Assets, Backend, CAPTURE_SCHEMA_V1, CaptureDescriptor, CaptureError, CaptureOptions,
    CapturePayloadKind, CaptureRevisions, Color, FramingOptions, GeometryDesc, MaterialDesc,
    NodeKey, PerspectiveCamera, PlatformSurface, ReferenceImageTolerance, RenderMode,
    RenderReadbackMode, Renderer, RendererOptions, Scene, SurfaceEvent, Tonemapper, Transform,
    Vec3, capture_contact_sheet_rgba8, capture_rgba8, capture_rgba8_from_pixels,
    capture_unverified_rgba8_from_pixels, compare_captures_with_tolerance, headless_gltf_viewer,
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
            camera: inspection.revisions.camera,
            appearance: inspection.revisions.appearance,
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
    assert_eq!(
        capture.descriptor.frame.pixel_source,
        "renderer_owned_readback"
    );
    assert_eq!(
        capture.descriptor.frame.state_binding,
        "exact_readback_completion"
    );
    assert!(capture.descriptor.frame.release_evidence);
    assert_eq!(
        capture.descriptor.frame.output_color_space,
        scena::OutputColorSpace::Srgb
    );
    assert!(
        capture
            .descriptor
            .frame
            .readback_completed_unix_ms
            .is_some()
    );
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
fn capture_from_supplied_rgba8_uses_supplied_pixels_and_rendered_state() {
    let (_assets, scene, renderer) = rendered_box_scene(1, 1);
    let rgba8 = vec![4, 5, 6, 255];

    let capture = capture_unverified_rgba8_from_pixels(
        &scene,
        &renderer,
        CaptureOptions::default(),
        1,
        1,
        rgba8.clone(),
    )
    .expect("capture from supplied pixels succeeds");

    assert_eq!(capture.rgba8, rgba8);
    assert!(!capture.descriptor.frame.release_evidence);
    assert_eq!(capture.descriptor.frame.pixel_source, "caller_supplied");
    assert_eq!(capture.descriptor.payload.byte_length, 4);
    assert_eq!(capture.descriptor.pixels.nonblack, 1);
    assert_eq!(capture.descriptor.pixels.center, [4, 5, 6, 255]);
    assert_eq!(
        capture.descriptor.pixels.fnv1a64,
        scena::fnv1a64_hex(capture.rgba8.as_slice())
    );
}

#[test]
fn capture_rgba8_encodes_and_writes_png_with_descriptor_dimensions() {
    let (_assets, scene, renderer) = rendered_box_scene(32, 24);
    let capture =
        capture_rgba8(&scene, &renderer, CaptureOptions::default()).expect("capture succeeds");

    let png_bytes = capture.to_png_bytes().expect("capture encodes PNG");
    let decoded = decode_png_rgba8(&png_bytes);

    assert_eq!(decoded.width, capture.descriptor.width);
    assert_eq!(decoded.height, capture.descriptor.height);
    assert_eq!(decoded.rgba8, capture.rgba8);

    let artifact = artifact_path("capture-rgba8-shared-api.png");
    capture.write_png(&artifact).expect("capture writes PNG");
    assert_eq!(
        std::fs::read(&artifact).expect("PNG artifact reads"),
        png_bytes
    );
}

#[test]
fn renderer_capture_png_delegates_to_capture_descriptor_path() {
    let (_assets, scene, renderer) = rendered_box_scene(40, 28);

    let capture = renderer
        .capture_rgba8(&scene, CaptureOptions::default())
        .expect("capture succeeds");
    let png_bytes = renderer
        .capture_png_bytes(&scene, CaptureOptions::default())
        .expect("renderer encodes PNG from capture");

    assert_eq!(
        decode_png_rgba8(&png_bytes).rgba8,
        capture.rgba8,
        "renderer PNG bytes must be encoded from the same descriptor-bound capture"
    );

    let artifact = artifact_path("renderer-capture-shared-api.png");
    renderer
        .capture_png(&scene, CaptureOptions::default(), &artifact)
        .expect("renderer writes PNG");
    assert_eq!(
        std::fs::read(&artifact).expect("renderer PNG artifact reads"),
        png_bytes
    );
}

#[test]
fn capture_contact_sheet_and_baseline_reports_record_capture_metadata() {
    let (_assets, scene, renderer) = rendered_box_scene(16, 12);
    let first = capture_rgba8(&scene, &renderer, CaptureOptions::default()).expect("first capture");
    let second =
        capture_rgba8(&scene, &renderer, CaptureOptions::default()).expect("second capture");

    let sheet = capture_contact_sheet_rgba8(&[first.clone(), second.clone()], 2)
        .expect("contact sheet builds");
    assert_eq!(sheet.width(), 32);
    assert_eq!(sheet.height(), 12);
    assert_eq!(sheet.tiles().len(), 2);
    assert_eq!(sheet.tiles()[0].descriptor.schema, CAPTURE_SCHEMA_V1);
    assert_eq!(
        decode_png_rgba8(&sheet.to_png_bytes().expect("sheet PNG")).width,
        32
    );

    let report = compare_captures_with_tolerance(&first, &second, ReferenceImageTolerance::exact())
        .expect("identical captures match");
    assert_eq!(report.schema, "scena.capture_baseline.v1");
    assert_eq!(report.status, "passed");
    assert_eq!(report.actual.schema, CAPTURE_SCHEMA_V1);
    assert_eq!(report.expected.schema, CAPTURE_SCHEMA_V1);
    assert_eq!(report.tolerance.max_abs_diff, 0);
    assert_eq!(report.diff.mismatched_pixels, 0);
    assert_eq!(report.actual.capabilities.backend, Backend::Headless);
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
fn capture_rejects_pixels_when_the_latest_render_has_no_matching_readback() {
    let (assets, mut scene, _cpu_renderer, mesh) = box_scene_with_camera_and_mesh(32, 32);
    let mut renderer = Renderer::headless_gpu(32, 32)
        .expect("C03 provenance contract requires the remote builder GPU adapter");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene A prepares");
    let camera = scene.active_camera().expect("camera exists");
    renderer
        .render_with_readback_mode(&scene, camera, RenderReadbackMode::Synchronous)
        .expect("scene A renders with readback");
    let stale_a = renderer.read_pixels().into_rgba8();

    scene
        .set_transform(mesh, Transform::at(Vec3::new(0.5, 0.0, 0.0)))
        .expect("scene B mutates");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene B prepares");
    renderer
        .render_with_readback_mode(&scene, camera, RenderReadbackMode::PresentOnly)
        .expect("scene B renders without readback");

    let error = capture_rgba8_from_pixels(
        &scene,
        &renderer,
        CaptureOptions::default(),
        32,
        32,
        stale_a,
    )
    .expect_err("A pixels must not be certified with B rendered state");
    assert!(
        error.to_string().contains("readback"),
        "error must explain that no pixel readback matches the rendered frame: {error}",
    );
}

#[test]
fn unverified_capture_accepts_caller_pixels_after_present_only_render() {
    let (assets, mut scene, _cpu_renderer, _mesh) = box_scene_with_camera_and_mesh(32, 24);
    let mut renderer = Renderer::headless_gpu(32, 24)
        .expect("C03 provenance contract requires the remote builder GPU adapter");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    let camera = scene.active_camera().expect("camera exists");
    renderer
        .render_with_readback_mode(&scene, camera, RenderReadbackMode::PresentOnly)
        .expect("scene renders without renderer-owned readback");

    let capture = capture_unverified_rgba8_from_pixels(
        &scene,
        &renderer,
        CaptureOptions::default(),
        32,
        24,
        vec![0; 32 * 24 * 4],
    )
    .expect("caller-owned browser canvas pixels remain available as non-release evidence");

    assert!(!capture.descriptor.frame.release_evidence);
    assert_eq!(capture.descriptor.frame.pixel_source, "caller_supplied");
    assert_eq!(
        capture.descriptor.frame.state_binding,
        "unverified_caller_supplied"
    );
    assert_eq!(capture.descriptor.frame.readback_completed_unix_ms, None);
}

#[test]
fn capture_rejects_pixels_swapped_from_an_older_readback_with_matching_dimensions() {
    let (assets, mut scene, mut renderer, mesh) = rendered_box_scene_with_mesh(32, 32);
    let camera = scene.active_camera().expect("active camera");
    let pixels_a = renderer.read_pixels().into_rgba8();
    scene
        .set_transform(mesh, Transform::at(Vec3::new(0.4, 0.0, 0.0)))
        .expect("mesh remains present");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene B prepares");
    renderer
        .render_with_readback_mode(&scene, camera, RenderReadbackMode::Synchronous)
        .expect("scene B renders with readback");

    let error = capture_rgba8_from_pixels(
        &scene,
        &renderer,
        CaptureOptions::default(),
        32,
        32,
        pixels_a,
    )
    .expect_err("scene A pixels must not be certified with scene B readback state");
    assert!(
        matches!(error, CaptureError::PixelReadbackMismatch { .. }),
        "swapped pixels need a structured mismatch error: {error}"
    );
}

#[test]
fn capture_provenance_survives_skips_and_invalidates_on_output_resize_and_loss() {
    let (assets, mut scene, _unused, _mesh) = box_scene_with_camera_and_mesh(32, 24);
    let mut renderer = Renderer::headless_with_options(
        32,
        24,
        RendererOptions::default().with_render_mode(RenderMode::OnChange),
    )
    .expect("on-change renderer builds");
    renderer.prepare_with_assets(&mut scene, &assets).unwrap();
    renderer.render_active(&scene).unwrap();
    let first = capture_rgba8(&scene, &renderer, CaptureOptions::default()).unwrap();
    let skipped = renderer
        .render_active(&scene)
        .expect("unchanged frame skips");
    assert!(skipped.skipped);
    let after_skip = capture_rgba8(&scene, &renderer, CaptureOptions::default()).unwrap();
    assert_eq!(after_skip.rgba8, first.rgba8);
    assert_eq!(
        after_skip.descriptor.frame.render_generation,
        first.descriptor.frame.render_generation
    );

    renderer.set_tonemapper(Tonemapper::Aces);
    assert!(matches!(
        capture_rgba8(&scene, &renderer, CaptureOptions::default()),
        Err(CaptureError::NoRenderedFrame)
    ));
    renderer.render_active(&scene).unwrap();
    let aces = capture_rgba8(&scene, &renderer, CaptureOptions::default()).unwrap();
    assert_eq!(aces.descriptor.frame.tonemapper, "aces");

    renderer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 40,
            height: 30,
        })
        .unwrap();
    assert!(matches!(
        capture_rgba8(&scene, &renderer, CaptureOptions::default()),
        Err(CaptureError::NoRenderedFrame)
    ));
    renderer.prepare_with_assets(&mut scene, &assets).unwrap();
    renderer.render_active(&scene).unwrap();
    let resized = capture_rgba8(&scene, &renderer, CaptureOptions::default()).unwrap();
    assert_eq!(
        (resized.descriptor.width, resized.descriptor.height),
        (40, 30)
    );

    renderer.handle_surface_event(SurfaceEvent::Lost).unwrap();
    assert!(matches!(
        capture_rgba8(&scene, &renderer, CaptureOptions::default()),
        Err(CaptureError::NoRenderedFrame)
    ));
    renderer
        .recover_surface(PlatformSurface::native_window(40, 30))
        .expect("descriptor surface recovers");
    renderer.prepare_with_assets(&mut scene, &assets).unwrap();
    renderer.render_active(&scene).unwrap();
    assert!(capture_rgba8(&scene, &renderer, CaptureOptions::default()).is_ok());

    renderer
        .handle_surface_event(SurfaceEvent::DeviceLost { recoverable: true })
        .unwrap();
    assert!(matches!(
        capture_rgba8(&scene, &renderer, CaptureOptions::default()),
        Err(CaptureError::NoRenderedFrame)
    ));
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
            camera: inspection.revisions.camera,
            appearance: inspection.revisions.appearance,
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

    let png_bytes = host
        .capture_png_bytes()
        .expect("host capture encodes descriptor-bound PNG");
    let decoded = decode_png_rgba8(&png_bytes);
    assert_eq!(decoded.width, 64);
    assert_eq!(decoded.height, 64);
    assert_eq!(decoded.rgba8, capture.rgba8);
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

#[derive(Debug)]
struct DecodedPng {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}

fn decode_png_rgba8(bytes: &[u8]) -> DecodedPng {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("PNG header reads");
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .expect("PNG output buffer size is known")
    ];
    let info = reader.next_frame(&mut buffer).expect("PNG payload reads");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    DecodedPng {
        width: info.width,
        height: info.height,
        rgba8: buffer[..info.buffer_size()].to_vec(),
    }
}

fn artifact_path(name: &str) -> PathBuf {
    let dir = PathBuf::from("target/gate-artifacts/capture-contracts");
    std::fs::create_dir_all(&dir).expect("artifact dir");
    dir.join(name)
}
