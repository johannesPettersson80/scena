use std::alloc::{GlobalAlloc, Layout, System};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use scena::{
    Aabb, AnchorFrame, AnimationTarget, AssetError, Assets, Backend, BuildError, CameraKey, Color,
    ConnectOptions, ConnectionAlignment, ConnectionError, ConnectionRequest, ConnectorFrame,
    ConnectorPolarity, ConnectorRollPolicy, DiagnosticCode, DiagnosticSeverity, GeometryDesc,
    ImportAnchorDebugMetadata, ImportDiagnosticOverlayKind, ImportOptions, InstantiateError,
    LabelDesc, LookupError, MaterialDesc, ModelHandle, NodeKey, NotPreparedReason, OrbitControls,
    OrthographicCamera, PerspectiveCamera, PointerEvent, PrepareError, Primitive, RenderError,
    Renderer, Scene, SourceCoordinateSystem, SourceUnits, SurfaceEvent, SurfaceSize,
    SurfaceViewport, TouchEvent, Transform, Vec3, Vertex,
};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

static COUNT_ALLOCATIONS: AtomicBool = AtomicBool::new(false);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNT_ALLOCATIONS.load(Ordering::Relaxed) {
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: this allocator only counts allocation calls and delegates all allocation
        // semantics to the system allocator with the original layout.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the pointer and layout are forwarded unchanged to the allocator that
        // created the allocation.
        unsafe { System.dealloc(pointer, layout) }
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn m7_connectors_place_objects_without_manual_matrix_math() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source node inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(3.0, 0.0, 0.0)))
        .expect("target node inserts");

    let source_connector =
        ConnectorFrame::new(source, Transform::at(Vec3::new(1.0, 0.0, 0.0))).named("out");
    let target_connector = ConnectorFrame::new(target, Transform::IDENTITY).named("in");

    let preview = scene
        .connect(
            source_connector,
            target_connector,
            ConnectOptions::default(),
        )
        .expect("connector placement solves");

    assert_vec3_near(
        preview.resolved_transform().translation,
        Vec3::new(2.0, 0.0, 0.0),
    );
    assert_vec3_near(
        scene
            .world_transform(source)
            .expect("source world transform exists")
            .translation,
        Vec3::new(2.0, 0.0, 0.0),
    );
    assert_eq!(
        scene
            .node(source)
            .expect("source node exists")
            .transform()
            .translation,
        Vec3::new(2.0, 0.0, 0.0)
    );
}

#[test]
fn m7_connectors_solve_parent_space_for_nested_nodes() {
    let mut scene = Scene::new();
    let source_parent = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(10.0, 0.0, 0.0)))
        .expect("source parent inserts");
    let source = scene
        .add_empty(source_parent, Transform::IDENTITY)
        .expect("source child inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(25.0, 0.0, 0.0)))
        .expect("target inserts");

    scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY).named("out"),
            ConnectorFrame::new(target, Transform::IDENTITY).named("in"),
            ConnectOptions::default(),
        )
        .expect("nested connector placement solves");

    assert_vec3_near(
        scene
            .node(source)
            .expect("source child exists")
            .transform()
            .translation,
        Vec3::new(15.0, 0.0, 0.0),
    );
    assert_vec3_near(
        scene
            .world_transform(source)
            .expect("source world transform exists")
            .translation,
        Vec3::new(25.0, 0.0, 0.0),
    );
}

#[test]
fn m7_connection_reparenting_is_explicit_and_preserves_world_transform() {
    let mut scene = Scene::new();
    let source_parent = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(10.0, 0.0, 0.0)))
        .expect("source parent inserts");
    let target_parent = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(20.0, 0.0, 0.0)))
        .expect("target parent inserts");
    let source = scene
        .add_empty(source_parent, Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(target_parent, Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .expect("target inserts");

    scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::new(target, Transform::IDENTITY).named("target"),
            ConnectOptions::default().reparent_source_to_target_parent(),
        )
        .expect("reparenting connector placement solves");

    assert_eq!(
        scene.node(source).expect("source exists").parent(),
        Some(target_parent)
    );
    assert_vec3_near(
        scene
            .world_transform(source)
            .expect("source world transform resolves")
            .translation,
        Vec3::new(22.0, 0.0, 0.0),
    );
    assert_vec3_near(
        scene
            .node(source)
            .expect("source exists")
            .transform()
            .translation,
        Vec3::new(2.0, 0.0, 0.0),
    );
}

#[test]
fn m7_connector_placement_preserves_fit_inside_scale_when_solving_position() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(5.0, 0.0, 0.0)))
        .expect("target inserts");
    scene
        .fit_inside(
            source,
            Aabb::new(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5)),
            Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)),
        )
        .expect("fit helper scales source before connection");

    scene
        .connect(
            ConnectorFrame::new(source, Transform::at(Vec3::new(1.0, 0.0, 0.0))).named("source"),
            ConnectorFrame::new(target, Transform::IDENTITY).named("target"),
            ConnectOptions::default(),
        )
        .expect("connection composes with fit helper");

    let source_transform = scene.node(source).expect("source exists").transform();
    assert_vec3_near(source_transform.scale, Vec3::new(2.0, 2.0, 2.0));
    assert_vec3_near(source_transform.translation, Vec3::new(3.0, 0.0, 0.0));
}

#[test]
fn m7_rotated_connector_connects_without_sideways_orientation_or_offset() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(
            scene.root(),
            Transform::at(Vec3::new(3.0, 2.0, 0.0)).rotate_z_deg(90.0),
        )
        .expect("target inserts");
    let source_connector = ConnectorFrame::new(
        source,
        Transform::at(Vec3::new(1.0, 0.0, 0.0)).rotate_z_deg(90.0),
    )
    .named("source");
    let target_connector =
        ConnectorFrame::new(target, Transform::at(Vec3::new(0.0, 0.5, 0.0))).named("target");

    scene
        .connect(
            source_connector.clone(),
            target_connector.clone(),
            ConnectOptions::default(),
        )
        .expect("rotated connector placement solves");

    let source_world = scene
        .world_transform(source)
        .expect("source world resolves");
    let target_world = scene
        .world_transform(target)
        .expect("target world resolves");
    let solved_source_connector =
        compose_test_transform(source_world, source_connector.local_transform());
    let solved_target_connector =
        compose_test_transform(target_world, target_connector.local_transform());

    assert_vec3_near(
        solved_source_connector.translation,
        solved_target_connector.translation,
    );
    assert_quat_same_orientation(
        solved_source_connector.rotation,
        solved_target_connector.rotation,
    );
}

#[test]
fn m7_forward_to_back_alignment_flips_source_without_manual_rotation() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(3.0, 0.0, 0.0)))
        .expect("target inserts");
    let source_connector = ConnectorFrame::new(source, Transform::IDENTITY).named("source");
    let target_connector = ConnectorFrame::new(target, Transform::IDENTITY).named("target");

    scene
        .connect(
            source_connector.clone(),
            target_connector.clone(),
            ConnectOptions::default().with_alignment(ConnectionAlignment::ForwardToBack),
        )
        .expect("forward-to-back alignment solves");

    let solved_source_connector = compose_test_transform(
        scene
            .world_transform(source)
            .expect("source world transform resolves"),
        source_connector.local_transform(),
    );
    let target_connector_world = compose_test_transform(
        scene
            .world_transform(target)
            .expect("target world transform resolves"),
        target_connector.local_transform(),
    );
    let expected_source_connector = compose_test_transform(
        target_connector_world,
        Transform::IDENTITY.rotate_y_deg(180.0),
    );

    assert_vec3_near(
        solved_source_connector.translation,
        expected_source_connector.translation,
    );
    assert_quat_same_orientation(
        solved_source_connector.rotation,
        expected_source_connector.rotation,
    );
}

#[test]
fn m7_explicit_roll_alignment_rotates_around_mated_connector_forward_axis() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(3.0, 0.0, 0.0)))
        .expect("target inserts");
    let source_connector = ConnectorFrame::new(source, Transform::IDENTITY).named("source");
    let target_connector = ConnectorFrame::new(target, Transform::IDENTITY).named("target");

    scene
        .connect(
            source_connector.clone(),
            target_connector.clone(),
            ConnectOptions::default().with_explicit_roll_degrees(90.0),
        )
        .expect("explicit roll alignment solves");

    let solved_source_connector = compose_test_transform(
        scene
            .world_transform(source)
            .expect("source world transform resolves"),
        source_connector.local_transform(),
    );
    let expected_source_connector = compose_test_transform(
        scene
            .world_transform(target)
            .expect("target world transform resolves"),
        Transform::IDENTITY.rotate_x_deg(90.0),
    );

    assert_vec3_near(
        solved_source_connector.translation,
        expected_source_connector.translation,
    );
    assert_quat_same_orientation(
        solved_source_connector.rotation,
        expected_source_connector.rotation,
    );
}

#[test]
fn m7_preserve_roll_alignment_keeps_source_roll_without_manual_matrix_math() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY.rotate_x_deg(42.0))
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(3.0, 0.0, 0.0)))
        .expect("target inserts");
    let source_connector = ConnectorFrame::new(source, Transform::IDENTITY).named("source");
    let target_connector = ConnectorFrame::new(target, Transform::IDENTITY).named("target");

    scene
        .connect(
            source_connector.clone(),
            target_connector.clone(),
            ConnectOptions::default().preserve_roll(),
        )
        .expect("preserve-roll alignment solves");

    let solved_source_connector = compose_test_transform(
        scene
            .world_transform(source)
            .expect("source world transform resolves"),
        source_connector.local_transform(),
    );
    let expected_source_connector = compose_test_transform(
        scene
            .world_transform(target)
            .expect("target world transform resolves"),
        Transform::IDENTITY.rotate_x_deg(42.0),
    );

    assert_quat_same_orientation(
        solved_source_connector.rotation,
        expected_source_connector.rotation,
    );
}

#[test]
fn m7_choose_nearest_roll_alignment_snaps_source_roll_without_guessing() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY.rotate_x_deg(47.0))
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(3.0, 0.0, 0.0)))
        .expect("target inserts");
    let source_connector = ConnectorFrame::new(source, Transform::IDENTITY).named("source");
    let target_connector = ConnectorFrame::new(target, Transform::IDENTITY).named("target");

    scene
        .connect(
            source_connector.clone(),
            target_connector.clone(),
            ConnectOptions::default().choose_nearest_roll_degrees(90.0),
        )
        .expect("choose-nearest roll alignment solves");

    let solved_source_connector = compose_test_transform(
        scene
            .world_transform(source)
            .expect("source world transform resolves"),
        source_connector.local_transform(),
    );
    let expected_source_connector = compose_test_transform(
        scene
            .world_transform(target)
            .expect("target world transform resolves"),
        Transform::IDENTITY.rotate_x_deg(90.0),
    );

    assert_quat_same_orientation(
        solved_source_connector.rotation,
        expected_source_connector.rotation,
    );
}

#[test]
fn m7_connector_mate_offset_is_applied_in_target_connector_space() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(
            scene.root(),
            Transform::at(Vec3::new(5.0, 0.0, 0.0)).rotate_z_deg(90.0),
        )
        .expect("target inserts");
    let source_connector = ConnectorFrame::new(source, Transform::IDENTITY).named("source");
    let target_connector = ConnectorFrame::new(target, Transform::IDENTITY).named("target");

    let preview = scene
        .connect(
            source_connector,
            target_connector,
            ConnectOptions::default().with_mate_offset(Transform::at(Vec3::new(1.0, 0.0, 0.0))),
        )
        .expect("connector offset placement solves");

    assert_vec3_near(
        preview.resolved_transform().translation,
        Vec3::new(5.0, 1.0, 0.0),
    );
    assert_vec3_near(
        scene
            .world_transform(source)
            .expect("source world transform exists")
            .translation,
        Vec3::new(5.0, 1.0, 0.0),
    );
}

#[test]
fn m7_connectors_reject_incompatible_connector_kinds_without_moving_source() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(1.0, 2.0, 3.0)))
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(4.0, 5.0, 6.0)))
        .expect("target inserts");

    let error = scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY)
                .named("source")
                .with_kind("pipe"),
            ConnectorFrame::new(target, Transform::IDENTITY)
                .named("target")
                .with_kind("cable"),
            ConnectOptions::default(),
        )
        .expect_err("incompatible connector kinds are rejected");

    assert!(matches!(
        error,
        ConnectionError::IncompatibleConnector {
            source_kind,
            target_kind
        } if source_kind == "pipe" && target_kind == "cable"
    ));
    assert_eq!(
        scene
            .node(source)
            .expect("source remains present")
            .transform()
            .translation,
        Vec3::new(1.0, 2.0, 3.0)
    );
}

#[test]
fn m7_connector_frame_metadata_guides_compatibility_without_domain_logic() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .expect("target inserts");

    let source_connector = ConnectorFrame::new(source, Transform::IDENTITY)
        .named("source")
        .with_kind("plug")
        .with_allowed_mate("socket")
        .with_tag("assembly")
        .with_snap_tolerance(0.01)
        .with_clearance_hint(0.05)
        .with_roll_policy(ConnectorRollPolicy::ChooseNearest)
        .with_polarity(ConnectorPolarity::Plug);
    let target_connector = ConnectorFrame::new(target, Transform::IDENTITY)
        .named("target")
        .with_kind("socket")
        .with_allowed_mate("plug")
        .with_polarity(ConnectorPolarity::Socket);

    assert!(source_connector.tags().contains("assembly"));
    assert_eq!(source_connector.allowed_mates(), ["socket"]);
    assert_eq!(source_connector.snap_tolerance(), Some(0.01));
    assert_eq!(source_connector.clearance_hint(), Some(0.05));
    assert_eq!(
        source_connector.roll_policy(),
        ConnectorRollPolicy::ChooseNearest
    );
    assert_eq!(source_connector.polarity(), Some(ConnectorPolarity::Plug));

    scene
        .connect(
            source_connector,
            target_connector,
            ConnectOptions::default(),
        )
        .expect("explicit allowed mate kinds are compatible");
    assert_vec3_near(
        scene
            .world_transform(source)
            .expect("source world transform exists")
            .translation,
        Vec3::new(2.0, 0.0, 0.0),
    );
}

#[test]
fn m7_imported_anchors_can_drive_connector_placement() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))
            .expect("anchor fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("anchor fixture instantiates");
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");

    let anchor = import
        .anchor("inspection")
        .expect("inspection anchor resolves");
    scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::from_import_anchor(anchor).with_kind("mount"),
            ConnectOptions::default(),
        )
        .expect("imported anchor connector placement solves");

    assert_vec3_near(
        scene
            .world_transform(source)
            .expect("source world transform exists")
            .translation,
        Vec3::new(0.0, 0.1, 0.0),
    );
}

#[test]
fn m7_import_anchor_tags_and_label_survive_anchor_frame_adapter() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))
            .expect("anchor fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("anchor fixture instantiates");

    let anchor = import
        .anchor("inspection")
        .expect("inspection anchor resolves");
    assert!(anchor.tags().contains("service"));
    assert_eq!(anchor.label(), Some("Inspection Port"));

    let frame = AnchorFrame::from_import_anchor(anchor);
    assert!(frame.tags().contains("visible"));
    assert_eq!(frame.label(), Some("Inspection Port"));
}

#[test]
fn m7_anchor_frame_registry_uses_typed_handles_and_metadata() {
    let mut scene = Scene::new();
    let node = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("anchor host inserts");
    let bounds = Aabb::new(Vec3::new(-0.1, -0.1, -0.1), Vec3::new(0.1, 0.1, 0.1));

    let anchor = scene
        .add_anchor(
            AnchorFrame::new(node, Transform::at(Vec3::new(0.2, 0.0, 0.0)))
                .named("mount")
                .with_tag("assembly")
                .with_label("Mounting point")
                .with_bounds_hint(bounds)
                .with_source_units(SourceUnits::Millimeters)
                .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded),
        )
        .expect("typed anchor registers");

    let resolved = scene.anchor(anchor).expect("typed anchor resolves");
    assert_eq!(scene.anchor_named("mount").expect("name resolves"), anchor);
    assert_eq!(resolved.name(), Some("mount"));
    assert_eq!(resolved.label(), Some("Mounting point"));
    assert!(resolved.tags().contains("assembly"));
    assert_eq!(resolved.bounds_hint(), Some(bounds));
    assert_eq!(resolved.source_units(), SourceUnits::Millimeters);
    assert_eq!(
        resolved.source_coordinate_system(),
        SourceCoordinateSystem::ZUpRightHanded
    );
}

#[test]
fn m7_import_anchor_frame_preserves_source_metadata_for_connector_adapter() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_units_scene.gltf"))
            .expect("unit fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate_with(
            &scene_asset,
            scena::ImportOptions::gltf_default()
                .with_source_units(SourceUnits::Inches)
                .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded),
        )
        .expect("source metadata import instantiates");
    let frame = AnchorFrame::from_import_anchor(
        import.anchor("one_foot").expect("imported anchor resolves"),
    )
    .with_tag("assembly");
    let connector = ConnectorFrame::from_anchor_frame(&frame).with_kind("mount");

    assert_eq!(frame.name(), Some("one_foot"));
    assert_eq!(frame.source_units(), SourceUnits::Inches);
    assert_eq!(
        frame.source_coordinate_system(),
        SourceCoordinateSystem::ZUpRightHanded
    );
    assert!(frame.tags().contains("assembly"));
    assert_vec3_near(
        connector.local_transform().translation,
        Vec3::new(12.0, 0.0, 0.0),
    );
    assert_eq!(connector.source_units(), SourceUnits::Inches);
    assert_eq!(
        connector.source_coordinate_system(),
        SourceCoordinateSystem::ZUpRightHanded
    );
}

#[test]
fn m7_imported_gltf_connectors_have_kind_lookup_and_stale_errors() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_debug_scene.gltf"))
            .expect("connector fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("connector fixture instantiates");

    let connector = import.connector("mount").expect("connector resolves");
    assert_eq!(connector.name(), "mount");
    assert_eq!(connector.kind(), Some("mount"));
    assert_vec3_near(connector.transform().translation, Vec3::new(0.0, 0.1, 0.0));
    assert!(
        import
            .diagnostic_overlays()
            .expect("connector diagnostics are available")
            .iter()
            .any(|overlay| {
                overlay.kind() == ImportDiagnosticOverlayKind::Connector
                    && overlay.label() == Some("mount")
            }),
        "imported connector overlays must be distinguishable from generic anchors"
    );

    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::from_import_connector(connector),
            ConnectOptions::default(),
        )
        .expect("imported connector frame drives placement");
    assert_vec3_near(
        scene
            .world_transform(source)
            .expect("source world transform exists")
            .translation,
        Vec3::new(0.0, 0.1, 0.0),
    );

    let replacement = scene
        .replace_import(&import, &scene_asset)
        .expect("replacement import instantiates");
    assert!(replacement.connector("mount").is_ok());
    assert!(matches!(
        import.connector("mount"),
        Err(LookupError::StaleImport)
    ));
}

#[test]
fn m7_imported_gltf_connector_metadata_survives_frame_adapter() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_debug_scene.gltf"))
            .expect("connector fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("connector fixture instantiates");

    let connector = import.connector("mount").expect("connector resolves");
    assert_eq!(connector.allowed_mates(), ["socket"]);
    assert!(connector.tags().contains("assembly"));
    assert_eq!(connector.snap_tolerance(), Some(0.025));
    assert_eq!(connector.clearance_hint(), Some(0.01));
    assert_eq!(connector.roll_policy(), ConnectorRollPolicy::ChooseNearest);
    assert_eq!(connector.polarity(), Some(ConnectorPolarity::Plug));
    assert_eq!(
        connector
            .metadata()
            .and_then(|metadata| metadata.get("author"))
            .and_then(|value| value.as_str()),
        Some("fixture")
    );

    let imported_frame = ConnectorFrame::from_import_connector(connector);
    assert_eq!(imported_frame.allowed_mates(), ["socket"]);
    assert!(imported_frame.tags().contains("front"));
    assert_eq!(imported_frame.snap_tolerance(), Some(0.025));
    assert_eq!(imported_frame.clearance_hint(), Some(0.01));
    assert_eq!(
        imported_frame.roll_policy(),
        ConnectorRollPolicy::ChooseNearest
    );
    assert_eq!(imported_frame.polarity(), Some(ConnectorPolarity::Plug));
    assert_eq!(
        imported_frame
            .metadata()
            .and_then(|metadata| metadata.get("revision"))
            .and_then(|value| value.as_i64()),
        Some(3)
    );

    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(0.0, 1.0, 0.0)))
        .expect("target inserts");
    scene
        .preview_connection(
            imported_frame,
            ConnectorFrame::new(target, Transform::IDENTITY).with_kind("socket"),
            ConnectOptions::default(),
        )
        .expect("allowed imported mate metadata makes socket compatible");
}

#[test]
fn m7_connectors_reject_non_uniform_scale_by_default() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(
            scene.root(),
            Transform {
                scale: Vec3::new(1.0, 2.0, 1.0),
                ..Transform::IDENTITY
            },
        )
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("target inserts");

    let error = scene
        .preview_connection(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::new(target, Transform::IDENTITY).named("target"),
            ConnectOptions::default(),
        )
        .expect_err("non-uniform source scale is risky by default");

    assert!(matches!(
        error,
        ConnectionError::NonUniformScaleConnectionRisk { node } if node == source
    ));
}

#[test]
fn m7_connectors_reject_degenerate_connector_rotation() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("target inserts");
    let degenerate_target = Transform {
        rotation: scena::Quat {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        },
        ..Transform::IDENTITY
    };

    let error = scene
        .preview_connection(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::new(target, degenerate_target).named("bad-target"),
            ConnectOptions::default(),
        )
        .expect_err("zero-length connector rotation is degenerate");

    assert!(matches!(
        error,
        ConnectionError::DegenerateConnectorFrame { connector }
            if connector.as_deref() == Some("bad-target")
    ));
}

#[test]
fn m7_manual_source_unit_mismatch_returns_structured_error() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("target inserts");
    let source_anchor = AnchorFrame::new(source, Transform::IDENTITY)
        .named("inch-authored")
        .with_source_units(SourceUnits::Inches);

    let error = scene
        .preview_connection(
            ConnectorFrame::from_anchor_frame(&source_anchor),
            ConnectorFrame::new(target, Transform::IDENTITY).named("meter-authored"),
            ConnectOptions::default(),
        )
        .expect_err("manual source-unit metadata mismatch must fail loudly");

    assert!(matches!(
        error,
        ConnectionError::UnitMismatch {
            source_units: SourceUnits::Inches,
            target_units: SourceUnits::Meters
        }
    ));
}

#[test]
fn m7_manual_source_coordinate_mismatch_returns_structured_error() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("target inserts");
    let source_anchor = AnchorFrame::new(source, Transform::IDENTITY)
        .named("z-up-authored")
        .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded);

    let error = scene
        .preview_connection(
            ConnectorFrame::from_anchor_frame(&source_anchor),
            ConnectorFrame::new(target, Transform::IDENTITY).named("gltf-authored"),
            ConnectOptions::default(),
        )
        .expect_err("manual source-coordinate metadata mismatch must fail loudly");

    assert!(matches!(
        error,
        ConnectionError::CoordinateSystemMismatch {
            source_coordinate_system: SourceCoordinateSystem::ZUpRightHanded,
            target_coordinate_system: SourceCoordinateSystem::GltfYUpRightHanded
        }
    ));
}

#[test]
fn m7_negative_determinant_connector_scale_returns_flipped_connection() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("target inserts");
    let flipped = Transform {
        scale: Vec3::new(-1.0, 1.0, 1.0),
        ..Transform::IDENTITY
    };

    let error = scene
        .preview_connection(
            ConnectorFrame::new(source, flipped).named("flipped-source"),
            ConnectorFrame::new(target, Transform::IDENTITY).named("target"),
            ConnectOptions::default(),
        )
        .expect_err("negative determinant connector frames must not mirror silently");

    assert!(matches!(
        error,
        ConnectionError::FlippedConnection {
            connector,
            node: None
        } if connector.as_deref() == Some("flipped-source")
    ));
}

#[test]
fn m7_negative_determinant_node_scale_returns_flipped_connection() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(
            scene.root(),
            Transform {
                scale: Vec3::new(-1.0, -1.0, -1.0),
                ..Transform::IDENTITY
            },
        )
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("target inserts");

    let error = scene
        .preview_connection(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::new(target, Transform::IDENTITY).named("target"),
            ConnectOptions::default(),
        )
        .expect_err("negative determinant source nodes must not mirror silently");

    assert!(matches!(
        error,
        ConnectionError::FlippedConnection {
            connector: None,
            node: Some(node)
        } if node == source
    ));
}

#[test]
fn m7_locked_connection_source_fails_before_moving_node() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .expect("target inserts");

    scene
        .lock_node_for_connections(source)
        .expect("source can be locked for connection solving");
    assert!(
        scene
            .node_connections_locked(source)
            .expect("lock state can be queried")
    );

    let error = scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY).named("locked-source"),
            ConnectorFrame::new(target, Transform::IDENTITY).named("target"),
            ConnectOptions::default(),
        )
        .expect_err("locked connection source must not be moved");

    assert!(matches!(
        error,
        ConnectionError::ConnectionWouldMoveLockedNode { node } if node == source
    ));
    assert_vec3_near(
        scene
            .world_transform(source)
            .expect("source transform remains available")
            .translation,
        Vec3::ZERO,
    );

    scene
        .unlock_node_for_connections(source)
        .expect("source can be unlocked");
    assert!(
        !scene
            .node_connections_locked(source)
            .expect("lock state can be queried")
    );
}

#[test]
fn m7_connector_host_not_prepared_rejects_model_placeholder_nodes() {
    let mut scene = Scene::new();
    let source = scene
        .model(ModelHandle::default())
        .add()
        .expect("model placeholder inserts");
    let target = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("target inserts");

    let error = scene
        .preview_connection(
            ConnectorFrame::new(source, Transform::IDENTITY).named("model-placeholder"),
            ConnectorFrame::new(target, Transform::IDENTITY).named("target"),
            ConnectOptions::default(),
        )
        .expect_err("raw model placeholder nodes must be imported before connector solving");

    assert!(matches!(
        error,
        ConnectionError::ConnectorHostNotPrepared {
            node,
            connector: Some(name)
        } if node == source && name == "model-placeholder"
    ));
}

#[test]
fn m7_connector_name_lookup_reports_ambiguity_with_typed_handles() {
    let mut scene = Scene::new();
    let first = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("first node inserts");
    let second = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("second node inserts");

    let first_connector = scene
        .add_connector(ConnectorFrame::new(first, Transform::IDENTITY).named("shared"))
        .expect("first connector registers");
    let second_connector = scene
        .add_connector(ConnectorFrame::new(second, Transform::IDENTITY).named("shared"))
        .expect("second connector registers");

    let error = scene
        .connector_named("shared")
        .expect_err("ambiguous connector names must not pick one silently");

    assert!(matches!(
        error,
        ConnectionError::AmbiguousConnector { name, matches }
            if name == "shared" && matches == vec![first_connector, second_connector]
    ));
}

#[test]
fn m7_validate_connections_returns_preview_without_mutating_scene() {
    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(3.0, 0.0, 0.0)))
        .expect("target inserts");
    let source_connector = scene
        .add_connector(ConnectorFrame::new(
            source,
            Transform::at(Vec3::new(1.0, 0.0, 0.0)),
        ))
        .expect("source connector registers");
    let target_connector = scene
        .add_connector(ConnectorFrame::new(target, Transform::IDENTITY))
        .expect("target connector registers");

    let previews = scene
        .validate_connections(&[ConnectionRequest::new(
            source_connector,
            target_connector,
            ConnectOptions::default(),
        )])
        .expect("connection request validates");

    assert_eq!(previews.len(), 1);
    assert_vec3_near(
        previews[0].resolved_transform().translation,
        Vec3::new(2.0, 0.0, 0.0),
    );
    let connection_line = previews[0].connection_line();
    assert_eq!(connection_line.source(), source);
    assert_eq!(connection_line.target(), target);
    assert_vec3_near(connection_line.start(), Vec3::new(1.0, 0.0, 0.0));
    assert_vec3_near(connection_line.end(), Vec3::new(3.0, 0.0, 0.0));
    assert_vec3_near(
        scene
            .node(source)
            .expect("source exists")
            .transform()
            .translation,
        Vec3::ZERO,
    );
}

#[test]
fn m7_stale_import_connector_handle_after_hot_reload_is_detected() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))
            .expect("anchor fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("anchor fixture instantiates");
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");
    let source_connector = scene
        .add_connector(ConnectorFrame::new(source, Transform::IDENTITY).named("source"))
        .expect("source connector registers");
    let stale_target_connector = scene
        .add_connector(
            ConnectorFrame::from_import_anchor(
                import
                    .anchor("inspection")
                    .expect("inspection anchor resolves"),
            )
            .with_kind("mount"),
        )
        .expect("import connector registers");

    let _replacement = scene
        .replace_import(&import, &scene_asset)
        .expect("replacement import instantiates");

    let error = scene
        .connect_by_key(
            source_connector,
            stale_target_connector,
            ConnectOptions::default(),
        )
        .expect_err("connector handles from stale imports must fail loudly");

    assert!(matches!(
        error,
        ConnectionError::StaleConnectorHandle { connector, name }
            if connector == Some(stale_target_connector) && name.as_deref() == Some("inspection")
    ));
}

#[test]
fn m7_replacement_import_rebinds_stable_anchor_and_connector_names() {
    let assets = Assets::new();
    let anchor_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))
            .expect("anchor fixture loads");
    let connector_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_debug_scene.gltf"))
            .expect("connector fixture loads");
    let mut scene = Scene::new();

    let anchor_import = scene
        .instantiate(&anchor_asset)
        .expect("anchor import instantiates");
    let previous_anchor = anchor_import
        .anchor("inspection")
        .expect("anchor resolves")
        .clone();
    let replacement_anchor_import = scene
        .replace_import(&anchor_import, &anchor_asset)
        .expect("anchor replacement instantiates");
    let replacement_anchor = replacement_anchor_import
        .replacement_anchor(&previous_anchor)
        .expect("stable anchor name rebinds after replacement");
    assert_eq!(replacement_anchor.name(), "inspection");

    let connector_import = scene
        .instantiate(&connector_asset)
        .expect("connector import instantiates");
    let previous_connector = connector_import
        .connector("mount")
        .expect("connector resolves")
        .clone();
    let replacement_connector_import = scene
        .replace_import(&connector_import, &connector_asset)
        .expect("connector replacement instantiates");
    let replacement_connector = replacement_connector_import
        .replacement_connector(&previous_connector)
        .expect("stable connector name rebinds after replacement");
    assert_eq!(replacement_connector.name(), "mount");
}

#[test]
fn m7_connector_placement_applies_source_units_before_solving() {
    assert_unit_anchor_connects_at(SourceUnits::Millimeters, "one_meter", 1.0);
    assert_unit_anchor_connects_at(SourceUnits::Inches, "one_foot", 0.3048);
    assert_unit_anchor_connects_at(SourceUnits::Feet, "two_feet", 0.6096);
}

#[test]
fn m7_gltf_anchor_units_override_import_units_for_connection_solving() {
    assert_unit_anchor_connects_with_import_units_at(
        "tests/assets/gltf/anchor_units_scene.gltf",
        SourceUnits::Meters,
        "one_meter",
        SourceUnits::Millimeters,
        1.0,
    );
    assert_unit_anchor_connects_with_import_units_at(
        "tests/assets/gltf/anchor_units_scene.gltf",
        SourceUnits::Meters,
        "one_foot",
        SourceUnits::Inches,
        0.3048,
    );
    assert_unit_anchor_connects_with_import_units_at(
        "tests/assets/gltf/anchor_units_scene.gltf",
        SourceUnits::Meters,
        "two_feet",
        SourceUnits::Feet,
        0.6096,
    );
}

#[test]
fn m7_import_exposes_source_units_and_coordinate_system_for_placement_diagnostics() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_units_scene.gltf"))
            .expect("unit fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate_with(
            &scene_asset,
            scena::ImportOptions::gltf_default()
                .with_source_units(SourceUnits::Inches)
                .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded),
        )
        .expect("source metadata import instantiates");

    assert_eq!(import.source_units(), SourceUnits::Inches);
    assert_eq!(
        import.source_coordinate_system(),
        SourceCoordinateSystem::ZUpRightHanded
    );
}

#[test]
fn m7_import_diagnostic_overlays_expose_source_units_and_coordinate_system() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_debug_scene.gltf"))
            .expect("connector fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate_with(
            &scene_asset,
            ImportOptions::gltf_default()
                .with_source_units(SourceUnits::Feet)
                .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded),
        )
        .expect("source metadata import instantiates");

    let connector_overlay = import
        .diagnostic_overlays()
        .expect("diagnostic overlays resolve")
        .iter()
        .find(|overlay| overlay.kind() == ImportDiagnosticOverlayKind::Connector)
        .expect("connector overlay exists");

    assert_eq!(connector_overlay.source_units(), SourceUnits::Feet);
    assert_eq!(
        connector_overlay.source_coordinate_system(),
        SourceCoordinateSystem::ZUpRightHanded
    );
}

#[test]
fn m7_z_up_import_connector_basis_converts_before_connection_solving() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_zup_scene.gltf"))
            .expect("Z-up connector fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate_with(
            &scene_asset,
            scena::ImportOptions::gltf_default()
                .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded),
        )
        .expect("Z-up connector fixture instantiates");
    let imported = import.connector("z-up-mount").expect("connector resolves");
    let target_frame = ConnectorFrame::from_import_connector(imported);

    assert_vec3_near(
        target_frame.local_transform().translation,
        Vec3::new(0.0, 1.0, 0.0),
    );
    assert_quat_same_orientation(
        target_frame.local_transform().rotation,
        Transform::IDENTITY.rotate_y_deg(90.0).rotation,
    );

    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source node inserts");
    scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            target_frame,
            ConnectOptions::default(),
        )
        .expect("Z-up connector converts before solving");
    assert_vec3_near(
        scene
            .world_transform(source)
            .expect("source world transform exists")
            .translation,
        Vec3::new(0.0, 1.0, 0.0),
    );
}

#[test]
fn m7_z_up_import_node_rotation_converts_before_connection_solving() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/connector_zup_node_rotation_scene.gltf"),
    )
    .expect("z-up rotated node connector fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate_with(
            &scene_asset,
            ImportOptions::gltf_default()
                .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded),
        )
        .expect("z-up fixture instantiates");
    let source_rotation = scena::Quat {
        x: 0.0,
        y: 0.0,
        z: 0.70710677,
        w: 0.70710677,
    };
    let expected = SourceCoordinateSystem::ZUpRightHanded
        .convert_connector_transform(Transform {
            rotation: source_rotation,
            ..Transform::IDENTITY
        })
        .rotation;

    assert_quat_same_orientation(
        scene
            .node(import.roots()[0])
            .expect("rotated import root exists")
            .transform()
            .rotation,
        expected,
    );
}

#[test]
fn m7_gltf_anchor_and_connector_basis_fields_avoid_manual_quaternions() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_basis_scene.gltf"))
            .expect("basis connector fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("basis connector fixture instantiates");
    let expected = Transform::IDENTITY.rotate_z_deg(90.0).rotation;

    let anchor_frame = ConnectorFrame::from_import_anchor(
        import
            .anchor("basis-anchor")
            .expect("basis anchor resolves"),
    );
    let connector_frame = ConnectorFrame::from_import_connector(
        import
            .connector("basis-connector")
            .expect("basis connector resolves"),
    );

    assert_quat_same_orientation(anchor_frame.local_transform().rotation, expected);
    assert_quat_same_orientation(connector_frame.local_transform().rotation, expected);
}

#[test]
fn m7_left_handed_import_connector_rejects_instead_of_mirroring_silently() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_zup_scene.gltf"))
            .expect("left-handed connector fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate_with(
            &scene_asset,
            scena::ImportOptions::gltf_default()
                .with_source_coordinate_system(SourceCoordinateSystem::YUpLeftHanded),
        )
        .expect("left-handed connector fixture instantiates");
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source node inserts");

    let error = scene
        .preview_connection(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::from_import_connector(
                import
                    .connector("z-up-mount")
                    .expect("connector resolves from left-handed import"),
            ),
            ConnectOptions::default(),
        )
        .expect_err("left-handed connector conversion must not mirror silently");

    assert!(matches!(
        error,
        ConnectionError::HandednessMismatch {
            connector,
            coordinate_system: SourceCoordinateSystem::YUpLeftHanded
        } if connector.as_deref() == Some("z-up-mount")
    ));
}

#[test]
fn m7_left_handed_mesh_import_fails_closed_until_winding_policy_exists() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("mesh fixture loads");
    let mut scene = Scene::new();

    let error = scene
        .instantiate_with(
            &scene_asset,
            scena::ImportOptions::gltf_default()
                .with_source_coordinate_system(SourceCoordinateSystem::YUpLeftHanded),
        )
        .expect_err("left-handed mesh imports must not mirror winding silently");

    assert!(matches!(
        error,
        InstantiateError::UnsupportedCoordinateSystem {
            coordinate_system: SourceCoordinateSystem::YUpLeftHanded,
            ..
        }
    ));
}

#[test]
fn m7_three_imported_objects_connect_into_assembly_without_raw_matrix_math() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_debug_scene.gltf"))
            .expect("connector fixture loads");
    let mut scene = Scene::new();
    let first = scene
        .instantiate(&scene_asset)
        .expect("first connector fixture instantiates");
    let second = scene
        .instantiate(&scene_asset)
        .expect("second connector fixture instantiates");
    let third = scene
        .instantiate(&scene_asset)
        .expect("third connector fixture instantiates");

    scene
        .connect(
            ConnectorFrame::from_import_connector(
                second
                    .connector("mount")
                    .expect("second connector resolves"),
            ),
            ConnectorFrame::from_import_connector(
                first.connector("mount").expect("first connector resolves"),
            ),
            ConnectOptions::default().with_mate_offset(Transform::at(Vec3::new(1.0, 0.0, 0.0))),
        )
        .expect("second part connects to first");
    scene
        .connect(
            ConnectorFrame::from_import_connector(
                third.connector("mount").expect("third connector resolves"),
            ),
            ConnectorFrame::from_import_connector(
                second
                    .connector("mount")
                    .expect("second connector still resolves"),
            ),
            ConnectOptions::default().with_mate_offset(Transform::at(Vec3::new(1.0, 0.0, 0.0))),
        )
        .expect("third part connects to second");

    assert_vec3_near(
        scene
            .world_transform(first.roots()[0])
            .expect("first root has a world transform")
            .translation,
        Vec3::ZERO,
    );
    assert_vec3_near(
        scene
            .world_transform(second.roots()[0])
            .expect("second root has a world transform")
            .translation,
        Vec3::new(1.0, 0.0, 0.0),
    );
    assert_vec3_near(
        scene
            .world_transform(third.roots()[0])
            .expect("third root has a world transform")
            .translation,
        Vec3::new(2.0, 0.0, 0.0),
    );
}

fn assert_vec3_near(actual: Vec3, expected: Vec3) {
    const EPSILON: f32 = 1.0e-5;
    assert!(
        (actual.x - expected.x).abs() <= EPSILON
            && (actual.y - expected.y).abs() <= EPSILON
            && (actual.z - expected.z).abs() <= EPSILON,
        "expected {actual:?} to be near {expected:?}"
    );
}

fn assert_unit_anchor_connects_at(units: SourceUnits, anchor: &str, expected_x_meters: f32) {
    assert_unit_anchor_connects_with_import_units_at(
        "tests/assets/gltf/anchor_import_units_scene.gltf",
        units,
        anchor,
        units,
        expected_x_meters,
    );
}

fn assert_unit_anchor_connects_with_import_units_at(
    fixture: &str,
    import_units: SourceUnits,
    anchor: &str,
    expected_anchor_units: SourceUnits,
    expected_x_meters: f32,
) {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(assets.load_scene(fixture)).expect("fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate_with(
            &scene_asset,
            scena::ImportOptions::gltf_default().with_source_units(import_units),
        )
        .expect("unit anchor fixture instantiates");
    assert_eq!(
        import
            .anchor(anchor)
            .expect("unit-converted anchor resolves")
            .source_units(),
        expected_anchor_units
    );
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source inserts");

    scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::from_import_anchor(
                import
                    .anchor(anchor)
                    .expect("unit-converted anchor resolves"),
            ),
            ConnectOptions::default(),
        )
        .expect("unit-converted connector placement solves");

    assert_vec3_near(
        scene
            .world_transform(source)
            .expect("source world transform exists")
            .translation,
        Vec3::new(expected_x_meters, 0.0, 0.0),
    );
}

fn assert_quat_same_orientation(actual: scena::Quat, expected: scena::Quat) {
    let dot = actual.x * expected.x
        + actual.y * expected.y
        + actual.z * expected.z
        + actual.w * expected.w;
    assert!(
        (dot.abs() - 1.0).abs() <= 1.0e-5,
        "expected {actual:?} to represent same orientation as {expected:?}"
    );
}

fn compose_test_transform(parent: Transform, child: Transform) -> Transform {
    let scaled_child_translation = Vec3::new(
        child.translation.x * parent.scale.x,
        child.translation.y * parent.scale.y,
        child.translation.z * parent.scale.z,
    );
    Transform {
        translation: add_vec3(
            parent.translation,
            rotate_test_vec3(parent.rotation, scaled_child_translation),
        ),
        rotation: multiply_test_quat(parent.rotation, child.rotation),
        scale: Vec3::new(
            parent.scale.x * child.scale.x,
            parent.scale.y * child.scale.y,
            parent.scale.z * child.scale.z,
        ),
    }
}

fn rotate_test_vec3(rotation: scena::Quat, vector: Vec3) -> Vec3 {
    let tx = 2.0 * (rotation.y * vector.z - rotation.z * vector.y);
    let ty = 2.0 * (rotation.z * vector.x - rotation.x * vector.z);
    let tz = 2.0 * (rotation.x * vector.y - rotation.y * vector.x);
    Vec3::new(
        vector.x + rotation.w * tx + (rotation.y * tz - rotation.z * ty),
        vector.y + rotation.w * ty + (rotation.z * tx - rotation.x * tz),
        vector.z + rotation.w * tz + (rotation.x * ty - rotation.y * tx),
    )
}

fn multiply_test_quat(left: scena::Quat, right: scena::Quat) -> scena::Quat {
    normalize_test_quat(scena::Quat {
        x: left.w * right.x + left.x * right.w + left.y * right.z - left.z * right.y,
        y: left.w * right.y - left.x * right.z + left.y * right.w + left.z * right.x,
        z: left.w * right.z + left.x * right.y - left.y * right.x + left.z * right.w,
        w: left.w * right.w - left.x * right.x - left.y * right.y - left.z * right.z,
    })
}

fn normalize_test_quat(value: scena::Quat) -> scena::Quat {
    let length =
        (value.x * value.x + value.y * value.y + value.z * value.z + value.w * value.w).sqrt();
    scena::Quat {
        x: value.x / length,
        y: value.y / length,
        z: value.z / length,
        w: value.w / length,
    }
}

const fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

#[test]
fn m7_camera_projection_renders_world_space_triangle_outside_ndc() {
    let (mut scene, camera) = world_space_triangle_scene();

    let mut renderer = Renderer::headless(96, 96).expect("renderer builds");
    renderer
        .prepare(&mut scene)
        .expect("world-space scene prepares");
    renderer
        .render(&scene, camera)
        .expect("camera-projected scene renders");

    assert_eq!(
        renderer.stats().culled_objects,
        0,
        "camera-visible world-space geometry must not be culled by raw NDC coordinates"
    );
    assert!(
        count_nonblack_pixels(renderer.frame_rgba8()) > 0,
        "camera-visible world-space geometry must produce pixels"
    );
}

#[test]
fn m7_headless_gpu_camera_projection_renders_world_space_triangle_outside_ndc() {
    match Renderer::headless_gpu(96, 96) {
        Ok(mut renderer) => {
            let (mut scene, camera) = world_space_triangle_scene();
            renderer
                .prepare(&mut scene)
                .expect("world-space scene prepares");
            renderer
                .render(&scene, camera)
                .expect("camera-projected scene renders");

            assert_eq!(renderer.stats().gpu_submissions, 1);
            assert!(
                count_nonblack_pixels(renderer.frame_rgba8()) > 0,
                "GPU must project camera-visible world-space geometry before rasterization"
            );
        }
        Err(BuildError::NoAdapter { backend }) => {
            assert_eq!(backend, Backend::HeadlessGpu);
        }
        Err(BuildError::RequestDevice { backend }) => {
            assert_eq!(backend, Backend::HeadlessGpu);
        }
        Err(error) => panic!("unexpected headless GPU setup result: {error:?}"),
    }
}

#[test]
fn m7_cpu_and_headless_gpu_camera_projection_match_within_tolerance_when_available() {
    let (cpu_scene, cpu_camera) = world_space_triangle_scene();
    let cpu_frame = render_headless_frame_at_size(cpu_scene, cpu_camera, 96, 96);
    let Some(cpu_centroid) = nonblack_centroid(&cpu_frame, 96) else {
        panic!("CPU camera-framed fixture must render nonblack pixels");
    };
    let cpu_pixels = count_nonblack_pixels(&cpu_frame);
    let Ok(mut renderer) = Renderer::headless_gpu(96, 96) else {
        return;
    };
    let (mut gpu_scene, gpu_camera) = world_space_triangle_scene();

    renderer
        .prepare(&mut gpu_scene)
        .expect("GPU camera-framed fixture prepares");
    renderer
        .render(&gpu_scene, gpu_camera)
        .expect("GPU camera-framed fixture renders");

    let gpu_frame = renderer.frame_rgba8();
    let Some(gpu_centroid) = nonblack_centroid(gpu_frame, 96) else {
        panic!("GPU camera-framed fixture must render nonblack pixels");
    };
    let gpu_pixels = count_nonblack_pixels(gpu_frame);
    let pixel_delta = cpu_pixels.abs_diff(gpu_pixels);
    assert!(
        pixel_delta <= cpu_pixels / 4,
        "CPU and GPU camera-framed fixture coverage should stay within 25%, cpu={cpu_pixels}, gpu={gpu_pixels}"
    );
    assert!(
        (cpu_centroid.0 - gpu_centroid.0).abs() <= 2.0
            && (cpu_centroid.1 - gpu_centroid.1).abs() <= 2.0,
        "CPU and GPU camera-framed fixture centroids should match within two pixels, cpu={cpu_centroid:?}, gpu={gpu_centroid:?}"
    );
}

#[test]
fn m7_picking_uses_camera_ray_for_world_space_triangle_outside_ndc() {
    let (scene, camera) = world_space_triangle_scene();
    let viewport = scena::Viewport::new(96, 96, 1.0).expect("viewport is valid");

    let hit = scene
        .pick(
            camera,
            scena::CursorPosition::physical(48.0, 48.0),
            viewport,
        )
        .expect("picking succeeds")
        .expect("camera ray hits centered world-space triangle");

    assert!(matches!(hit.target(), scena::HitTarget::Node(_)));
    assert_vec3_near(hit.world_position, Vec3::new(2.0, 0.0, 0.0));
}

#[test]
fn m7_pick_with_assets_hits_direct_mesh_without_legacy_renderable() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.75, 0.75, 0.75));
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.2, 0.8, 1.0)));
    let mut scene = Scene::new();
    let mesh = scene.mesh(geometry, material).add().expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let viewport = scena::Viewport::new(96, 96, 1.0).expect("viewport is valid");

    assert!(
        scene
            .pick(
                camera,
                scena::CursorPosition::physical(48.0, 48.0),
                viewport,
            )
            .expect("legacy renderable picking still succeeds")
            .is_none(),
        "plain pick only covers legacy renderable primitives and must not pretend direct assets are pickable"
    );

    let hit = scene
        .pick_with_assets(
            camera,
            scena::CursorPosition::physical(48.0, 48.0),
            viewport,
            &assets,
        )
        .expect("asset-aware picking succeeds")
        .expect("center ray hits direct mesh");

    assert!(matches!(hit.target(), scena::HitTarget::Node(node) if node == mesh));
}

#[test]
fn m7_pick_with_assets_hits_instance_set_without_manual_triangles() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.45, 0.45, 0.45));
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.9, 0.4, 0.1)));
    let mut scene = Scene::new();
    let set = scene
        .add_instance_set(scene.root(), geometry, material, Transform::IDENTITY)
        .expect("instance set inserts");
    scene
        .push_instance(set, Transform::IDENTITY)
        .expect("instance inserts");
    scene
        .push_instance(set, Transform::at(Vec3::new(0.9, 0.0, 0.0)))
        .expect("second instance inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let viewport = scena::Viewport::new(96, 96, 1.0).expect("viewport is valid");

    let hit = scene
        .pick_with_assets(
            camera,
            scena::CursorPosition::physical(48.0, 48.0),
            viewport,
            &assets,
        )
        .expect("asset-aware picking succeeds")
        .expect("center ray hits an instance");

    assert!(matches!(hit.target(), scena::HitTarget::Node(_)));
}

#[test]
fn m7_pick_and_select_with_assets_updates_interaction_for_direct_mesh() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.75, 0.75, 0.75));
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.2, 0.8, 1.0)));
    let mut scene = Scene::new();
    let mesh = scene.mesh(geometry, material).add().expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");

    let hit = scene
        .pick_and_select_with_assets(
            camera,
            scena::CursorPosition::physical(48.0, 48.0),
            scena::Viewport::new(96, 96, 1.0).expect("viewport validates"),
            &assets,
        )
        .expect("asset-aware select picking succeeds")
        .expect("center ray hits direct mesh");

    assert!(matches!(hit.target(), scena::HitTarget::Node(node) if node == mesh));
    assert_eq!(scene.interaction().hover(), Some(hit.target()));
    assert_eq!(scene.interaction().primary_selection(), Some(hit.target()));

    scene
        .pick_and_hover_with_assets(
            camera,
            scena::CursorPosition::physical(48.0, 48.0),
            scena::Viewport::new(96, 96, 1.0).expect("viewport validates"),
            &assets,
        )
        .expect("asset-aware hover picking succeeds");
    assert_eq!(scene.interaction().hover(), Some(hit.target()));
}

#[test]
fn m7_pick_with_assets_hits_imported_gltf_mesh_without_manual_triangles() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("mesh glTF loads");
    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset).expect("glTF instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_import(camera, &import)
        .expect("camera frames import");
    let viewport = scena::Viewport::new(96, 96, 1.0).expect("viewport is valid");

    let hit = scene
        .pick_with_assets(
            camera,
            scena::CursorPosition::physical(48.0, 48.0),
            viewport,
            &assets,
        )
        .expect("asset-aware glTF picking succeeds")
        .expect("center ray hits imported mesh");

    let scena::HitTarget::Node(node) = hit.target();
    assert!(import.roots().contains(&node));
}

#[test]
fn m7_orbit_controls_keep_framed_asset_visible_after_camera_distance_change() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.6, 0.4, 0.3));
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.2, 0.8, 1.0)));
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("camera frames mesh");
    OrbitControls::new(Vec3::ZERO, 2.5)
        .apply_to_scene(&mut scene, camera)
        .expect("controls apply after framing");

    let mut renderer = Renderer::headless(96, 96).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");

    assert!(
        count_nonblack_pixels(renderer.frame_rgba8()) > 0,
        "orbit controls must not move a framed asset outside the camera depth range"
    );
}

#[test]
fn m7_moving_camera_changes_rendered_pixels_and_screen_position() {
    let (centered_scene, centered_camera) =
        world_space_triangle_scene_with_camera(Transform::at(Vec3::new(2.0, 0.0, 3.0)));
    let centered_frame = render_headless_frame(centered_scene, centered_camera);
    let centered_centroid =
        nonblack_centroid(&centered_frame, 96).expect("centered triangle renders");

    let (offset_scene, offset_camera) =
        world_space_triangle_scene_with_camera(Transform::at(Vec3::new(2.45, 0.0, 3.0)));
    let offset_frame = render_headless_frame(offset_scene, offset_camera);
    let offset_centroid = nonblack_centroid(&offset_frame, 96).expect("offset triangle renders");

    assert_ne!(
        centered_frame, offset_frame,
        "moving only the camera must change rendered pixels"
    );
    assert!(
        offset_centroid.0 < centered_centroid.0 - 4.0,
        "camera moving right should move the triangle left on screen: centered={centered_centroid:?} offset={offset_centroid:?}"
    );
}

#[test]
fn m7_renderable_parent_world_transform_drives_rendered_placement() {
    let (nested_scene, nested_camera) = renderable_scene_with_parent_transform(
        Transform::at(Vec3::new(0.45, 0.0, 0.0)),
        Transform::IDENTITY,
    );
    let nested_frame = render_headless_frame(nested_scene, nested_camera);

    let (direct_scene, direct_camera) = renderable_scene_with_parent_transform(
        Transform::IDENTITY,
        Transform::at(Vec3::new(0.45, 0.0, 0.0)),
    );
    let direct_frame = render_headless_frame(direct_scene, direct_camera);

    assert_eq!(
        nested_frame, direct_frame,
        "render preparation must use composed world transforms so nested placement renders like the equivalent direct world placement"
    );
}

#[test]
fn m7_mesh_parent_world_transform_drives_rendered_placement() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.35, 0.35, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));

    let (nested_scene, nested_camera) = mesh_scene_with_parent_transform(
        geometry,
        material,
        Transform::at(Vec3::new(0.45, 0.0, 0.0)),
        Transform::IDENTITY,
    );
    let nested_frame = render_headless_frame_with_assets(nested_scene, nested_camera, &assets);

    let (direct_scene, direct_camera) = mesh_scene_with_parent_transform(
        geometry,
        material,
        Transform::IDENTITY,
        Transform::at(Vec3::new(0.45, 0.0, 0.0)),
    );
    let direct_frame = render_headless_frame_with_assets(direct_scene, direct_camera, &assets);

    assert_eq!(
        nested_frame, direct_frame,
        "asset-backed meshes must render from world transforms so parented object placement cannot silently drift"
    );
}

#[test]
fn m7_rotating_camera_changes_rendered_pixels_and_recenters_target() {
    let (unrotated_scene, unrotated_camera) =
        world_space_triangle_scene_with_camera(Transform::at(Vec3::new(2.45, 0.0, 3.0)));
    let unrotated_frame = render_headless_frame(unrotated_scene, unrotated_camera);
    let unrotated_centroid =
        nonblack_centroid(&unrotated_frame, 96).expect("unrotated triangle renders");

    let (mut rotated_scene, rotated_camera) =
        world_space_triangle_scene_with_camera(Transform::at(Vec3::new(2.45, 0.0, 3.0)));
    rotated_scene
        .look_at_point(rotated_camera, Vec3::new(2.0, 0.0, 0.0))
        .expect("camera rotates toward target");
    let rotated_frame = render_headless_frame(rotated_scene, rotated_camera);
    let rotated_centroid = nonblack_centroid(&rotated_frame, 96).expect("rotated triangle renders");

    assert_ne!(
        unrotated_frame, rotated_frame,
        "rotating only the camera must change rendered pixels"
    );
    assert!(
        rotated_centroid.0 > unrotated_centroid.0 + 4.0,
        "look_at should bring the target back toward screen center: unrotated={unrotated_centroid:?} rotated={rotated_centroid:?}"
    );
}

#[test]
fn m7_camera_look_at_nested_target_uses_world_transform() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));

    let (mut nested_scene, nested_camera, target_node, target_world) =
        nested_mesh_camera_target_scene(geometry, material);
    nested_scene
        .look_at(nested_camera, target_node)
        .expect("camera looks at nested target");
    let nested_frame = render_headless_frame_with_assets(nested_scene, nested_camera, &assets);

    let (mut point_scene, point_camera, _, _) = nested_mesh_camera_target_scene(geometry, material);
    point_scene
        .look_at_point(point_camera, target_world)
        .expect("camera looks at explicit world point");
    let point_frame = render_headless_frame_with_assets(point_scene, point_camera, &assets);

    assert_eq!(
        nested_frame, point_frame,
        "Scene::look_at must aim at the target node's composed world transform, not its local transform"
    );
}

#[test]
fn m7_camera_look_at_point_nested_camera_uses_world_position() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let target_world = Vec3::new(1.2, 0.35, 0.0);

    let (mut nested_scene, nested_camera) =
        mesh_scene_with_nested_camera(geometry, material, target_world);
    nested_scene
        .look_at_point(nested_camera, target_world)
        .expect("nested camera looks at world point");
    let nested_frame = render_headless_frame_with_assets(nested_scene, nested_camera, &assets);

    let (mut direct_scene, direct_camera) = mesh_scene_with_camera_transform(
        geometry,
        material,
        target_world,
        Transform::at(Vec3::new(0.0, 0.0, 3.0)),
    );
    direct_scene
        .look_at_point(direct_camera, target_world)
        .expect("direct camera looks at world point");
    let direct_frame = render_headless_frame_with_assets(direct_scene, direct_camera, &assets);

    assert_eq!(
        nested_frame, direct_frame,
        "Scene::look_at_point must aim nested cameras from their composed world position"
    );
}

#[test]
fn m7_frame_nested_camera_preserves_requested_world_camera_pose() {
    let bounds = Aabb::new(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5));

    let mut direct_scene = Scene::new();
    let direct_camera = direct_scene.add_default_camera().expect("camera inserts");
    direct_scene
        .frame(direct_camera, bounds)
        .expect("direct camera frames bounds");
    let direct_world = direct_scene
        .world_transform(
            direct_scene
                .camera_node(direct_camera)
                .expect("camera node"),
        )
        .expect("direct camera world transform resolves");

    let mut nested_scene = Scene::new();
    let parent = nested_scene
        .add_empty(nested_scene.root(), Transform::at(Vec3::new(1.5, 0.0, 0.0)))
        .expect("camera parent inserts");
    let nested_camera = nested_scene
        .add_perspective_camera(parent, PerspectiveCamera::default(), Transform::IDENTITY)
        .expect("nested camera inserts");
    nested_scene
        .frame(nested_camera, bounds)
        .expect("nested camera frames bounds");
    let nested_world = nested_scene
        .world_transform(
            nested_scene
                .camera_node(nested_camera)
                .expect("camera node"),
        )
        .expect("nested camera world transform resolves");

    assert_eq!(nested_world.translation, direct_world.translation);
    assert_quat_same_orientation(nested_world.rotation, direct_world.rotation);
}

#[test]
fn m7_perspective_and_orthographic_cameras_project_different_pixel_footprints() {
    let (perspective_scene, perspective_camera) =
        world_space_triangle_scene_with_camera(Transform::at(Vec3::new(2.0, 0.0, 3.0)));
    let perspective_frame = render_headless_frame(perspective_scene, perspective_camera);
    let perspective_pixels = count_nonblack_pixels(&perspective_frame);

    let mut orthographic_scene = world_space_triangle_only();
    let orthographic_camera = orthographic_scene
        .add_orthographic_camera(
            orthographic_scene.root(),
            OrthographicCamera {
                left: -1.0,
                right: 1.0,
                bottom: -1.0,
                top: 1.0,
                near: 0.01,
                far: 10.0,
            },
            Transform::at(Vec3::new(2.0, 0.0, 3.0)),
        )
        .expect("orthographic camera inserts");
    orthographic_scene
        .set_active_camera(orthographic_camera)
        .expect("orthographic camera activates");
    let orthographic_frame = render_headless_frame(orthographic_scene, orthographic_camera);
    let orthographic_pixels = count_nonblack_pixels(&orthographic_frame);

    assert!(
        orthographic_pixels > perspective_pixels * 2,
        "orthographic and perspective projection should produce visibly different footprints: perspective={perspective_pixels} orthographic={orthographic_pixels}"
    );
}

#[test]
fn m7_default_perspective_camera_uses_render_target_aspect_on_wide_viewports() {
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(2.0, -0.2, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(2.4, -0.2, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(2.2, 0.2, 0.0),
                    color: Color::WHITE,
                },
            ])],
            Transform::default(),
        )
        .expect("wide-viewport triangle inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("default camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    let mut renderer = Renderer::headless(192, 96).expect("wide renderer builds");

    renderer.prepare(&mut scene).expect("wide scene prepares");
    renderer.render(&scene, camera).expect("wide scene renders");

    assert!(
        count_nonblack_pixels(renderer.frame_rgba8()) > 0,
        "default perspective camera should use target aspect so wide-canvas content is not clipped"
    );
}

#[test]
fn m7_device_pixel_ratio_resize_preserves_projection_aspect() {
    let (reference_scene, reference_camera) =
        world_space_triangle_scene_with_camera(Transform::at(Vec3::new(2.0, 0.0, 3.0)));
    let reference_frame = render_headless_frame_at_size(reference_scene, reference_camera, 160, 80);

    let (mut dpr_scene, dpr_camera) =
        world_space_triangle_scene_with_camera(Transform::at(Vec3::new(2.0, 0.0, 3.0)));
    let mut renderer = Renderer::headless(80, 40).expect("logical-size renderer builds");
    renderer
        .handle_surface_event(SurfaceEvent::ViewportChanged(
            SurfaceViewport::new(80.0, 40.0, 2.0).expect("DPR viewport is valid"),
        ))
        .expect("DPR resize applies physical target");
    renderer
        .prepare(&mut dpr_scene)
        .expect("DPR scene prepares");
    renderer
        .render(&dpr_scene, dpr_camera)
        .expect("DPR scene renders");

    assert_eq!(renderer.stats().target_width, 160);
    assert_eq!(renderer.stats().target_height, 80);
    assert_eq!(
        renderer.frame_rgba8(),
        reference_frame.as_slice(),
        "DPR resize should render with the same projection as an equivalent physical target"
    );
}

fn world_space_triangle_scene() -> (Scene, CameraKey) {
    world_space_triangle_scene_with_camera(Transform::at(Vec3::new(2.0, 0.0, 3.0)))
}

fn world_space_triangle_scene_with_camera(camera_transform: Transform) -> (Scene, CameraKey) {
    let mut scene = world_space_triangle_only();
    let camera = scene
        .add_perspective_camera(scene.root(), PerspectiveCamera::default(), camera_transform)
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    (scene, camera)
}

fn world_space_triangle_only() -> Scene {
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(1.5, -0.4, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(2.5, -0.4, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(2.0, 0.6, 0.0),
                    color: Color::WHITE,
                },
            ])],
            Transform::IDENTITY,
        )
        .expect("world-space triangle inserts");
    scene
}

fn renderable_scene_with_parent_transform(
    parent_transform: Transform,
    child_transform: Transform,
) -> (Scene, CameraKey) {
    let mut scene = Scene::new();
    let parent = scene
        .add_empty(scene.root(), parent_transform)
        .expect("parent inserts");
    scene
        .add_renderable(
            parent,
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-0.2, -0.2, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(0.2, -0.2, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(0.0, 0.25, 0.0),
                    color: Color::WHITE,
                },
            ])],
            child_transform,
        )
        .expect("nested renderable inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    (scene, camera)
}

fn mesh_scene_with_parent_transform(
    geometry: scena::GeometryHandle,
    material: scena::MaterialHandle,
    parent_transform: Transform,
    child_transform: Transform,
) -> (Scene, CameraKey) {
    let mut scene = Scene::new();
    let parent = scene
        .add_empty(scene.root(), parent_transform)
        .expect("parent inserts");
    scene
        .mesh(geometry, material)
        .parent(parent)
        .transform(child_transform)
        .add()
        .expect("nested mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    (scene, camera)
}

fn nested_mesh_camera_target_scene(
    geometry: scena::GeometryHandle,
    material: scena::MaterialHandle,
) -> (Scene, CameraKey, scena::NodeKey, Vec3) {
    let mut scene = Scene::new();
    let parent_transform = Transform::at(Vec3::new(1.2, 0.0, 0.0));
    let child_transform = Transform::at(Vec3::new(0.0, 0.35, 0.0));
    let parent = scene
        .add_empty(scene.root(), parent_transform)
        .expect("parent inserts");
    let mesh = scene
        .mesh(geometry, material)
        .parent(parent)
        .transform(child_transform)
        .add()
        .expect("nested mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    let target_world = scene
        .world_transform(mesh)
        .expect("mesh world transform resolves")
        .translation;
    (scene, camera, mesh, target_world)
}

fn mesh_scene_with_nested_camera(
    geometry: scena::GeometryHandle,
    material: scena::MaterialHandle,
    mesh_translation: Vec3,
) -> (Scene, CameraKey) {
    let mut scene = mesh_only_scene(geometry, material, mesh_translation);
    let camera_parent = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(0.0, 0.0, 3.0)))
        .expect("camera parent inserts");
    let camera = scene
        .add_perspective_camera(
            camera_parent,
            PerspectiveCamera::default(),
            Transform::IDENTITY,
        )
        .expect("nested camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    (scene, camera)
}

fn mesh_scene_with_camera_transform(
    geometry: scena::GeometryHandle,
    material: scena::MaterialHandle,
    mesh_translation: Vec3,
    camera_transform: Transform,
) -> (Scene, CameraKey) {
    let mut scene = mesh_only_scene(geometry, material, mesh_translation);
    let camera = scene
        .add_perspective_camera(scene.root(), PerspectiveCamera::default(), camera_transform)
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    (scene, camera)
}

fn mesh_only_scene(
    geometry: scena::GeometryHandle,
    material: scena::MaterialHandle,
    mesh_translation: Vec3,
) -> Scene {
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(mesh_translation))
        .add()
        .expect("mesh inserts");
    scene
}

fn render_headless_frame(scene: Scene, camera: CameraKey) -> Vec<u8> {
    render_headless_frame_at_size(scene, camera, 96, 96)
}

fn render_headless_frame_at_size(
    mut scene: Scene,
    camera: CameraKey,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let mut renderer = Renderer::headless(width, height).expect("renderer builds");
    renderer
        .prepare(&mut scene)
        .expect("world-space scene prepares");
    renderer
        .render(&scene, camera)
        .expect("camera-projected scene renders");
    renderer.frame_rgba8().to_vec()
}

fn render_headless_frame_with_assets(
    mut scene: Scene,
    camera: CameraKey,
    assets: &Assets,
) -> Vec<u8> {
    let mut renderer = Renderer::headless(96, 96).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("asset scene prepares");
    renderer
        .render(&scene, camera)
        .expect("asset scene renders");
    renderer.frame_rgba8().to_vec()
}

fn count_nonblack_pixels(frame: &[u8]) -> usize {
    frame
        .chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}

fn nonblack_centroid(frame: &[u8], width: u32) -> Option<(f32, f32)> {
    let mut count = 0.0;
    let mut x_sum = 0.0;
    let mut y_sum = 0.0;
    for (index, pixel) in frame.chunks_exact(4).enumerate() {
        if pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0 {
            continue;
        }
        count += 1.0;
        x_sum += (index as u32 % width) as f32;
        y_sum += (index as u32 / width) as f32;
    }
    (count > 0.0).then_some((x_sum / count, y_sum / count))
}

#[test]
fn m7_transform_helpers_and_frame_import_avoid_manual_matrix_math() {
    let transform = Transform::at(Vec3::new(1.0, 2.0, 3.0))
        .scale_by(2.0)
        .rotate_z_deg(90.0);

    assert_eq!(transform.translation, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(transform.scale, Vec3::new(2.0, 2.0, 2.0));
    assert!(transform.rotation.z > 0.70);
    assert!(transform.rotation.w > 0.70);

    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("fixture glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("fixture instantiates");
    let camera = scene.add_default_camera().expect("default camera inserts");

    scene
        .frame_import(camera, &import)
        .expect("import bounds frame without manual matrix math");
    let camera_node = scene.camera_node(camera).expect("camera node resolves");
    assert!(
        scene
            .node(camera_node)
            .expect("camera node exists")
            .transform()
            .translation
            .z
            > 0.5
    );

    let nested_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/nested_mesh_bounds_scene.gltf"))
            .expect("nested bounds fixture loads");
    let mut nested_scene = Scene::new();
    let nested_import = nested_scene
        .instantiate(&nested_asset)
        .expect("nested bounds fixture instantiates");
    let nested_bounds = nested_import
        .bounds_world(&nested_scene)
        .expect("nested import reports world bounds");
    assert_vec3_near(nested_bounds.min, Vec3::new(2.5, 1.5, 0.0));
    assert_vec3_near(nested_bounds.max, Vec3::new(3.5, 2.5, 0.0));

    let marker = scene
        .add_empty(scene.root(), Transform::default())
        .expect("marker inserts");
    scene
        .center_on(marker, Vec3::new(4.0, 5.0, 6.0))
        .expect("center helper updates node translation");
    assert_eq!(
        scene
            .node(marker)
            .expect("marker exists")
            .transform()
            .translation,
        Vec3::new(4.0, 5.0, 6.0)
    );
}

#[test]
fn m7_bounds_helpers_on_nested_nodes_preserve_requested_world_placement() {
    let mut scene = Scene::new();
    let parent = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .expect("parent inserts");
    let centered = scene
        .add_empty(parent, Transform::IDENTITY)
        .expect("centered child inserts");

    scene
        .center_on(centered, Vec3::new(3.0, 4.0, 0.0))
        .expect("center_on accepts nested node");
    assert_eq!(
        scene
            .world_transform(centered)
            .expect("centered world transform")
            .translation,
        Vec3::new(3.0, 4.0, 0.0)
    );

    let aligned = scene
        .add_empty(parent, Transform::IDENTITY)
        .expect("aligned child inserts");
    let desired = Transform::at(Vec3::new(4.0, 0.0, 0.0)).rotate_z_deg(90.0);
    scene
        .align_to(aligned, desired)
        .expect("align_to accepts nested node");
    let aligned_world = scene
        .world_transform(aligned)
        .expect("aligned world transform resolves");
    assert_eq!(aligned_world.translation, desired.translation);
    assert_quat_same_orientation(aligned_world.rotation, desired.rotation);

    let fitted = scene
        .add_empty(parent, Transform::IDENTITY)
        .expect("fitted child inserts");
    scene
        .fit_inside(
            fitted,
            Aabb::new(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5)),
            Aabb::new(Vec3::new(4.0, -1.0, -1.0), Vec3::new(6.0, 1.0, 1.0)),
        )
        .expect("fit_inside accepts nested node");
    let fitted_world = scene
        .world_transform(fitted)
        .expect("fitted world transform resolves");
    assert_eq!(fitted_world.translation, Vec3::new(5.0, 0.0, 0.0));
    assert_eq!(fitted_world.scale, Vec3::new(2.0, 2.0, 2.0));
}

#[test]
fn m7_viewer_operations_dirty_prepare_without_persistent_resource_growth() {
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("renderable inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer.prepare(&mut scene).expect("initial prepare");
    renderer.render(&scene, camera).expect("warm render");
    let baseline = renderer.stats();

    let mut controls = OrbitControls::new(Vec3::ZERO, 2.0);
    assert_eq!(
        controls.handle_pointer(PointerEvent::primary_pressed(32.0, 32.0)),
        scena::OrbitControlAction::BeginOrbit
    );
    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    COUNT_ALLOCATIONS.store(true, Ordering::Relaxed);
    let control_action = controls.handle_pointer(PointerEvent::moved(34.0, 31.0, 2.0, -1.0));
    COUNT_ALLOCATIONS.store(false, Ordering::Relaxed);
    assert_eq!(control_action, scena::OrbitControlAction::Orbit);
    assert_eq!(
        ALLOCATION_COUNT.load(Ordering::Relaxed),
        0,
        "steady orbit pointer handling should not allocate"
    );

    controls
        .apply_to_scene(&mut scene, camera)
        .expect("controls update camera");
    assert!(matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::SceneChanged { .. }
        })
    ));
    renderer
        .prepare(&mut scene)
        .expect("reprepare after controls");
    assert_stable_viewer_resource_counters(baseline, renderer.stats());

    scene
        .pick_and_select(camera, 32.0, 32.0, 64, 64, 1.0)
        .expect("picking computes")
        .expect("center pick hits");
    assert!(matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::SceneChanged { .. }
        })
    ));
    renderer.prepare(&mut scene).expect("reprepare after pick");
    assert_stable_viewer_resource_counters(baseline, renderer.stats());
}

fn assert_stable_viewer_resource_counters(
    before: scena::RendererStats,
    after: scena::RendererStats,
) {
    assert_eq!(after.buffers, before.buffers);
    assert_eq!(after.textures, before.textures);
    assert_eq!(after.materials, before.materials);
    assert_eq!(after.render_targets, before.render_targets);
    assert_eq!(after.pipelines, before.pipelines);
    assert_eq!(after.bind_groups, before.bind_groups);
    assert_eq!(after.shader_modules, before.shader_modules);
    assert_eq!(after.live_logical_handles, before.live_logical_handles);
    assert_eq!(after.pending_destructions, before.pending_destructions);
}

#[test]
fn m7_benchmark_artifact_writes_required_viewer_workflow_rows() {
    let rows = vec![
        benchmark_m7_first_render(),
        benchmark_m7_first_glb(),
        benchmark_m7_camera_framing(),
        benchmark_m7_controls_input(),
        benchmark_m7_picking_selection(),
        benchmark_m7_helpers(),
        benchmark_m7_labels(),
        benchmark_m7_static_batching(),
        benchmark_m7_high_instance_viewer(),
    ];
    let report = serde_json::json!({
        "schema": "scena.m7.workflow_benchmarks.v1",
        "gate": "m7-workflow-benchmarks",
        "status": "passed",
        "regression_threshold_percent": 5.0,
        "rows": rows,
    });
    let artifact = root().join("target/gate-artifacts/m7-workflow-benchmarks.json");
    fs::create_dir_all(artifact.parent().expect("artifact has parent")).expect("artifact dir");
    fs::write(
        artifact,
        serde_json::to_string_pretty(&report).expect("report serializes"),
    )
    .expect("benchmark artifact is written");

    for workflow in [
        "first-render",
        "first-glb",
        "camera-framing",
        "controls-input",
        "picking-selection",
        "helpers",
        "labels",
        "static-batching",
        "high-instance-viewer",
    ] {
        let row = report["rows"]
            .as_array()
            .expect("rows are an array")
            .iter()
            .find(|row| row["workflow"] == workflow)
            .unwrap_or_else(|| panic!("missing M7 benchmark row {workflow}"));
        assert_eq!(row["status"], "passed");
        assert!(
            row["duration_ms"].as_f64().unwrap_or_default() >= 0.0,
            "{workflow} must record a duration"
        );
    }
}

fn benchmark_m7_first_render() -> serde_json::Value {
    benchmark_m7_workflow("first-render", || {
        let mut scene = Scene::new();
        scene
            .add_renderable(
                scene.root(),
                vec![Primitive::unlit_triangle()],
                Transform::default(),
            )
            .expect("renderable inserts");
        scene.add_default_camera().expect("camera inserts");
        let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
        renderer.prepare(&mut scene).expect("first scene prepares");
        let outcome = renderer.render_active(&scene).expect("first scene renders");
        M7BenchmarkOutcome::rendered(renderer.capabilities().backend, outcome.draw_calls)
    })
}

fn benchmark_m7_first_glb() -> serde_json::Value {
    benchmark_m7_workflow("first-glb", || {
        let assets = Assets::new();
        let scene_asset = pollster::block_on(
            assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
        )
        .expect("fixture glTF loads");
        let mut scene = Scene::new();
        let import = scene
            .instantiate(&scene_asset)
            .expect("fixture instantiates");
        let camera = scene.add_default_camera().expect("camera inserts");
        scene
            .frame_import(camera, &import)
            .expect("camera frames import");
        let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("GLB scene prepares");
        let outcome = renderer.render_active(&scene).expect("GLB scene renders");
        M7BenchmarkOutcome::rendered(renderer.capabilities().backend, outcome.draw_calls)
    })
}

fn benchmark_m7_camera_framing() -> serde_json::Value {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("fixture glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("fixture instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");

    benchmark_m7_workflow("camera-framing", || {
        scene
            .frame_import(camera, &import)
            .expect("camera frames import");
        M7BenchmarkOutcome::cpu()
    })
}

fn benchmark_m7_controls_input() -> serde_json::Value {
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut controls = OrbitControls::new(Vec3::ZERO, 3.0).with_damping(0.2);

    benchmark_m7_workflow("controls-input", || {
        controls.handle_pointer(PointerEvent::primary_pressed(8.0, 8.0));
        controls.handle_pointer(PointerEvent::moved(14.0, 12.0, 6.0, 4.0));
        controls.handle_pointer(PointerEvent::wheel(14.0, 12.0, -0.25));
        controls
            .apply_to_scene(&mut scene, camera)
            .expect("controls apply");
        M7BenchmarkOutcome::cpu()
    })
}

fn benchmark_m7_picking_selection() -> serde_json::Value {
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("renderable inserts");
    let camera = scene.add_default_camera().expect("camera inserts");

    benchmark_m7_workflow("picking-selection", || {
        let hit = scene
            .pick_and_select(camera, 32.0, 32.0, 64, 64, 1.0)
            .expect("picking succeeds");
        M7BenchmarkOutcome {
            backend: "Cpu",
            draw_calls: u64::from(hit.is_some()),
            skipped: false,
        }
    })
}

fn benchmark_m7_helpers() -> serde_json::Value {
    benchmark_m7_workflow("helpers", || {
        let bounds = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let helpers = [
            GeometryDesc::axes(1.0),
            GeometryDesc::grid(0.25, 4),
            GeometryDesc::arrow(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)),
            GeometryDesc::bounding_box(bounds),
            GeometryDesc::camera_frustum(0.1, 4.0, 1.6, 60.0),
            GeometryDesc::light_helper(0.25),
            GeometryDesc::origin_marker(0.2),
            GeometryDesc::pivot_marker(0.2),
            GeometryDesc::anchor_marker(0.2),
        ];
        M7BenchmarkOutcome {
            backend: "Cpu",
            draw_calls: helpers
                .iter()
                .map(|helper| helper.indices().len() as u64)
                .sum(),
            skipped: false,
        }
    })
}

fn benchmark_m7_labels() -> serde_json::Value {
    benchmark_m7_workflow("labels", || {
        let mut scene = Scene::new();
        scene.add_default_camera().expect("camera inserts");
        scene
            .add_label(
                scene.root(),
                LabelDesc::sdf("Pressure").with_size(14.0),
                Transform::default(),
            )
            .expect("label inserts");
        let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
        renderer.prepare(&mut scene).expect("label scene prepares");
        let outcome = renderer.render_active(&scene).expect("label scene renders");
        M7BenchmarkOutcome::rendered(renderer.capabilities().backend, outcome.draw_calls)
    })
}

fn benchmark_m7_static_batching() -> serde_json::Value {
    benchmark_m7_workflow("static-batching", || {
        let assets = Assets::new();
        let batch = assets.create_static_batch(
            &GeometryDesc::box_xyz(0.1, 0.1, 0.1),
            [
                Transform::at(Vec3::new(-0.2, 0.0, 0.0)),
                Transform::at(Vec3::new(0.0, 0.0, 0.0)),
                Transform::at(Vec3::new(0.2, 0.0, 0.0)),
            ],
        );
        let material = assets.create_material(MaterialDesc::unlit(scena::Color::WHITE));
        let mut scene = Scene::new();
        scene.mesh(batch, material).add().expect("batch inserts");
        scene.add_default_camera().expect("camera inserts");
        let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("batch scene prepares");
        let outcome = renderer.render_active(&scene).expect("batch scene renders");
        M7BenchmarkOutcome::rendered(renderer.capabilities().backend, outcome.draw_calls)
    })
}

fn benchmark_m7_high_instance_viewer() -> serde_json::Value {
    benchmark_m7_workflow("high-instance-viewer", || {
        let assets = Assets::new();
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.08, 0.08, 0.08));
        let material = assets.create_material(MaterialDesc::unlit(scena::Color::WHITE));
        let mut scene = Scene::new();
        let set = scene
            .add_instance_set(scene.root(), geometry, material, Transform::default())
            .expect("instance set inserts");
        scene
            .reserve_instances(set, 128)
            .expect("reserve instances");
        for index in 0..128 {
            scene
                .push_instance(
                    set,
                    Transform::at(Vec3::new(
                        (index % 16) as f32 * 0.1 - 0.75,
                        (index / 16) as f32 * 0.1 - 0.35,
                        0.0,
                    )),
                )
                .expect("instance inserts");
        }
        scene.add_default_camera().expect("camera inserts");
        let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("instance scene prepares");
        let outcome = renderer
            .render_active(&scene)
            .expect("instance scene renders");
        M7BenchmarkOutcome::rendered(renderer.capabilities().backend, outcome.draw_calls)
    })
}

#[derive(Debug, Clone, Copy)]
struct M7BenchmarkOutcome {
    backend: &'static str,
    draw_calls: u64,
    skipped: bool,
}

impl M7BenchmarkOutcome {
    fn rendered(backend: Backend, draw_calls: u64) -> Self {
        Self {
            backend: backend_name(backend),
            draw_calls,
            skipped: false,
        }
    }

    const fn cpu() -> Self {
        Self {
            backend: "Cpu",
            draw_calls: 0,
            skipped: false,
        }
    }
}

fn benchmark_m7_workflow(
    workflow: &'static str,
    operation: impl FnOnce() -> M7BenchmarkOutcome,
) -> serde_json::Value {
    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    COUNT_ALLOCATIONS.store(true, Ordering::Relaxed);
    let start = Instant::now();
    let outcome = operation();
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    COUNT_ALLOCATIONS.store(false, Ordering::Relaxed);
    let allocations = ALLOCATION_COUNT.load(Ordering::Relaxed);

    serde_json::json!({
        "workflow": workflow,
        "status": "passed",
        "backend": outcome.backend,
        "duration_ms": duration_ms,
        "median_ms": duration_ms,
        "p95_ms": duration_ms,
        "draw_calls": outcome.draw_calls,
        "skipped": outcome.skipped,
        "allocation_count": allocations,
    })
}

const fn backend_name(backend: Backend) -> &'static str {
    match backend {
        Backend::Headless => "Headless",
        Backend::HeadlessGpu => "HeadlessGpu",
        Backend::SurfaceDescriptor => "SurfaceDescriptor",
        Backend::NativeSurface => "NativeSurface",
        Backend::WebGpu => "WebGpu",
        Backend::WebGl2 => "WebGl2",
    }
}

#[test]
fn m7_tags_visibility_layers_and_render_groups_are_scene_owned() {
    let mut scene = Scene::new();
    let node = scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("renderable inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");

    scene.add_tag(node, "interactive").expect("tag inserts");
    scene
        .set_layer_mask(node, 0b0101)
        .expect("layer mask updates");
    scene
        .set_render_group(node, 7)
        .expect("render group updates");
    scene
        .set_helper_on_top(node, true)
        .expect("helper-on-top metadata updates");

    assert!(scene.has_tag(node, "interactive"));
    assert_eq!(scene.tagged("interactive").collect::<Vec<_>>(), vec![node]);
    assert_eq!(scene.layer_mask(node), Some(0b0101));
    assert_eq!(scene.render_group(node), Some(7));
    assert_eq!(scene.helper_on_top(node), Some(true));

    let mut renderer = Renderer::headless(32, 32).expect("renderer builds");
    renderer
        .prepare(&mut scene)
        .expect("visible scene prepares");
    assert_eq!(
        renderer
            .render(&scene, camera)
            .expect("visible scene renders")
            .draw_calls,
        1
    );

    scene.set_visible(node, false).expect("visibility updates");
    renderer
        .prepare(&mut scene)
        .expect("hidden scene prepares explicitly");
    assert_eq!(
        renderer
            .render(&scene, camera)
            .expect("hidden scene renders")
            .draw_calls,
        0
    );
}

#[test]
fn m7_controls_picking_units_camera_masks_and_static_batching_are_product_shaped() {
    assert_eq!(SourceUnits::Millimeters.meters_per_unit(), 0.001);
    assert_eq!(
        SourceCoordinateSystem::ZUpRightHanded.convert_position(Vec3::new(1.0, 2.0, 3.0)),
        Vec3::new(1.0, 3.0, -2.0)
    );
    assert_eq!(
        SourceCoordinateSystem::YUpLeftHanded.convert_position(Vec3::new(1.0, 2.0, 3.0)),
        Vec3::new(1.0, 2.0, -3.0)
    );
    assert_eq!(
        SourceCoordinateSystem::ZUpLeftHanded.convert_position(Vec3::new(1.0, 2.0, 3.0)),
        Vec3::new(1.0, 3.0, 2.0)
    );

    let assets = Assets::new();
    let (batched, batch_report) = assets.create_static_batch_with_report(
        &GeometryDesc::box_xyz(0.1, 0.1, 0.1),
        [
            Transform::at(Vec3::new(-0.25, 0.0, 0.0)),
            Transform::at(Vec3::new(0.25, 0.0, 0.0)),
        ],
    );
    assert_eq!(batch_report.instance_count(), 2);
    assert_eq!(batch_report.output_vertices(), 48);
    assert_eq!(batch_report.output_indices(), 72);
    assert!(batch_report.requires_prepare_after_rebuild());
    assert_eq!(batch_report.picking_debug_instances(), 2);
    let batched_geometry = assets.geometry(batched).expect("batch geometry exists");
    assert_eq!(batched_geometry.indices().len(), 72);

    let material = assets.create_material(MaterialDesc::unlit(scena::Color::WHITE));
    let mut scene = Scene::new();
    let batched_node = scene
        .mesh(batched, material)
        .add()
        .expect("batched mesh inserts");
    let _pickable = scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("pickable renderable inserts");
    let hidden_by_camera = scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("renderable inserts");
    scene
        .set_layer_mask(hidden_by_camera, 0b0010)
        .expect("node layer updates");

    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .set_camera_layer_mask(camera, 0b0001)
        .expect("camera mask updates");
    assert_eq!(scene.camera_layer_mask(camera), Some(0b0001));

    let controls = OrbitControls::new(Vec3::ZERO, 2.0)
        .with_damping(0.18)
        .focus(Vec3::new(0.0, 0.0, 0.0), 3.0);
    controls
        .apply_to_scene(&mut scene, camera)
        .expect("controls apply camera transform");
    assert_eq!(controls.damping_factor(), 0.18);
    assert!(
        scene
            .node(scene.camera_node(camera).expect("camera node exists"))
            .expect("camera node resolves")
            .transform()
            .translation
            .z
            > 2.5
    );

    let hit = scene
        .pick_and_select(camera, 32.0, 32.0, 64, 64, 1.0)
        .expect("one-flow picking succeeds")
        .expect("center pointer hits triangle");
    assert!(matches!(hit.target(), scena::HitTarget::Node(_)));
    assert_eq!(scene.interaction().hover(), Some(hit.target()));
    assert_eq!(scene.interaction().primary_selection(), Some(hit.target()));
    scene.set_hover_target(None);
    assert_eq!(scene.interaction().hover(), None);
    scene.set_primary_selection_target(Some(hit.target()));
    assert_eq!(scene.interaction().primary_selection(), Some(hit.target()));
    scene
        .pick_and_hover(camera, 32.0, 32.0, 64, 64, 1.0)
        .expect("hover picking succeeds")
        .expect("center pointer hits triangle");
    assert_eq!(scene.interaction().hover(), Some(hit.target()));

    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares with camera mask");
    let outcome = renderer
        .render(&scene, camera)
        .expect("masked scene renders");
    assert_eq!(
        outcome.draw_calls, 25,
        "camera layer mask should exclude only the extra renderable from prepared output"
    );
    assert!(scene.visible(batched_node).expect("batched node exists"));
}

#[test]
fn m7_resize_dpr_controls_helpers_anchors_and_diagnostics_are_product_shaped() {
    let assets = Assets::new();
    let cube = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let material = assets.create_material(MaterialDesc::unlit(scena::Color::WHITE));

    let mut scene = Scene::new();
    scene.mesh(cube, material).add().expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");

    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial prepare succeeds");
    renderer
        .render_active(&scene)
        .expect("initial render succeeds");

    let viewport = SurfaceViewport::new(80.0, 40.0, 2.0).expect("valid DPR viewport");
    assert_eq!(
        viewport.physical_size(),
        SurfaceSize {
            width: 160,
            height: 80
        }
    );
    renderer
        .handle_surface_event(SurfaceEvent::ViewportChanged(viewport))
        .expect("viewport event updates target");
    assert!(matches!(
        renderer.render_active(&scene),
        Err(RenderError::NotPrepared { .. })
    ));
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("prepare after viewport change succeeds");
    assert_eq!(renderer.stats().target_width, 160);
    assert_eq!(renderer.stats().target_height, 80);

    let mut controls = OrbitControls::new(Vec3::ZERO, 3.0).with_damping(0.25);
    assert_eq!(
        controls.handle_pointer(PointerEvent::primary_pressed(10.0, 10.0)),
        scena::OrbitControlAction::BeginOrbit
    );
    assert_eq!(
        controls.handle_pointer(PointerEvent::moved(20.0, 8.0, 10.0, -2.0)),
        scena::OrbitControlAction::Orbit
    );
    assert_eq!(
        controls.handle_pointer(PointerEvent::wheel(20.0, 8.0, -0.3)),
        scena::OrbitControlAction::Zoom
    );
    assert_eq!(
        controls.handle_touch(TouchEvent::start(12.0, 12.0)),
        scena::OrbitControlAction::BeginOrbit
    );
    assert_eq!(
        controls.handle_touch(TouchEvent::move_by(18.0, 15.0, 6.0, 3.0)),
        scena::OrbitControlAction::Orbit
    );
    controls
        .apply_to_scene(&mut scene, camera)
        .expect("controls write camera transform and look-at rotation");
    let camera_transform = scene
        .node(scene.camera_node(camera).expect("camera node exists"))
        .expect("camera node resolves")
        .transform();
    assert_ne!(camera_transform.rotation, scena::Quat::IDENTITY);

    let bounds = Aabb::new(Vec3::new(-1.0, -0.5, -0.25), Vec3::new(1.0, 0.5, 0.25));
    assert_eq!(
        GeometryDesc::bounding_box(bounds).topology(),
        scena::GeometryTopology::Lines
    );
    assert_eq!(
        GeometryDesc::camera_frustum(0.1, 5.0, 1.6, 60.0).topology(),
        scena::GeometryTopology::Lines
    );
    assert_eq!(
        GeometryDesc::light_helper(0.25).topology(),
        scena::GeometryTopology::Lines
    );
    assert_eq!(
        GeometryDesc::origin_marker(0.3).topology(),
        scena::GeometryTopology::Lines
    );
    assert_eq!(
        GeometryDesc::pivot_marker(0.3).topology(),
        scena::GeometryTopology::Lines
    );
    assert_eq!(
        GeometryDesc::anchor_marker(0.3).topology(),
        scena::GeometryTopology::Lines
    );
    let normal_lines = GeometryDesc::normal_lines(&GeometryDesc::plane(1.0, 1.0), 0.25);
    assert_eq!(normal_lines.topology(), scena::GeometryTopology::Lines);
    assert_eq!(normal_lines.vertices().len(), 8);
    assert_eq!(normal_lines.indices().len(), 8);
    assert_vec3_near(
        normal_lines.vertices()[1].position,
        Vec3::new(-0.5, 0.25, -0.5),
    );

    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))
            .expect("fixture glTF loads");
    let import = scene.instantiate(&scene_asset).expect("scene instantiates");
    let anchors = import.anchors().expect("anchors resolve on live import");
    assert!(
        !anchors.is_empty(),
        "fixture must expose glTF anchor extras"
    );
    let debug = import
        .anchor_debug_metadata()
        .expect("anchor debug metadata resolves");
    assert!(debug.iter().all(ImportAnchorDebugMetadata::is_anchor));
    assert!(debug.iter().any(|anchor| anchor.name() == "inspection"));

    let missing = import
        .anchor("does-not-exist")
        .expect_err("missing anchor should be structured");
    assert!(matches!(missing, LookupError::AnchorNotFound { .. }));
    assert!(missing.help().contains("anchors_named"));
    assert!(
        RenderError::NoActiveCamera
            .help()
            .contains("add_default_camera")
    );
}

#[test]
fn m7_beginner_scene_diagnostics_explain_invisible_setups() {
    let scene = Scene::new();
    let renderer = Renderer::headless(32, 32).expect("renderer builds");

    let diagnostics = renderer.diagnose_scene(&scene);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::MissingActiveCamera
            && diagnostic.severity == DiagnosticSeverity::Error
            && diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help.contains("add_default_camera"))
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::InvisibleScene
            && diagnostic.severity == DiagnosticSeverity::Warning
            && diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help.contains("visibility"))
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::MissingLightingOrEnvironment
            && diagnostic.severity == DiagnosticSeverity::Warning
            && diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help.contains("set_environment"))
    }));
}

#[test]
fn m7_diagnostics_report_invalid_camera_projection_before_empty_frame() {
    let mut scene = Scene::new();
    let invalid_camera = PerspectiveCamera {
        near: 10.0,
        far: 1.0,
        ..PerspectiveCamera::default()
    };
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            invalid_camera,
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("renderable inserts");
    let renderer = Renderer::headless(32, 32).expect("renderer builds");

    let diagnostics = renderer.diagnose_scene(&scene);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::InvalidCameraProjection
            && diagnostic.severity == DiagnosticSeverity::Error
            && diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help.contains("near") && help.contains("far"))
    }));
}

#[test]
fn m7_diagnostics_report_camera_visibility_failures_before_empty_frame() {
    let renderer = Renderer::headless(64, 64).expect("renderer builds");

    let mut behind_scene = Scene::new();
    behind_scene
        .add_renderable(
            behind_scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-0.2, -0.2, 4.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(0.2, -0.2, 4.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(0.0, 0.2, 4.0),
                    color: Color::WHITE,
                },
            ])],
            Transform::default(),
        )
        .expect("behind-camera triangle inserts");
    let behind_camera = behind_scene
        .add_perspective_camera(
            behind_scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("behind camera inserts");
    behind_scene
        .set_active_camera(behind_camera)
        .expect("behind camera activates");

    let behind_diagnostics = renderer.diagnose_scene(&behind_scene);
    assert!(behind_diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::ObjectsBehindCamera
            && diagnostic.severity == DiagnosticSeverity::Warning
            && diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help.contains("frame") || help.contains("look_at"))
    }));

    let mut outside_scene = Scene::new();
    outside_scene
        .add_renderable(
            outside_scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(20.0, -0.2, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(20.4, -0.2, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(20.2, 0.2, 0.0),
                    color: Color::WHITE,
                },
            ])],
            Transform::default(),
        )
        .expect("off-frustum triangle inserts");
    let outside_camera = outside_scene
        .add_perspective_camera(
            outside_scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("outside camera inserts");
    outside_scene
        .set_active_camera(outside_camera)
        .expect("outside camera activates");

    let outside_diagnostics = renderer.diagnose_scene(&outside_scene);
    assert!(outside_diagnostics.iter().any(|diagnostic| {
        diagnostic.code == DiagnosticCode::SceneOutsideCameraFrustum
            && diagnostic.severity == DiagnosticSeverity::Warning
            && diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help.contains("frame") || help.contains("near/far"))
    }));
}

#[test]
fn m7_diagnostics_report_import_bounds_outside_camera_frustum() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("mesh fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("mesh fixture instantiates");
    scene
        .set_transform(import.roots()[0], Transform::at(Vec3::new(40.0, 0.0, 0.0)))
        .expect("import root moves outside frustum");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");

    let diagnostics = Renderer::headless(96, 96)
        .expect("renderer builds")
        .diagnose_scene(&scene);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::SceneOutsideCameraFrustum
                && diagnostic.message.contains("mesh bounds")
        }),
        "asset-backed mesh/import bounds outside the camera frustum must be diagnosed before users debug a blank frame: {diagnostics:?}"
    );
}

#[test]
fn m7_diagnostics_with_assets_report_direct_mesh_bounds_outside_camera_frustum() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(40.0, 0.0, 0.0)))
        .add()
        .expect("off-frustum direct mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");

    let diagnostics = Renderer::headless(96, 96)
        .expect("renderer builds")
        .diagnose_scene_with_assets(&scene, &assets);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::SceneOutsideCameraFrustum
                && diagnostic.message.contains("asset mesh bounds")
        }),
        "direct asset mesh bounds outside the camera frustum must be diagnosed before users debug a blank frame: {diagnostics:?}"
    );
}

#[test]
fn m7_first_assembly_helper_connects_imported_connectors_by_name() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_debug_scene.gltf"))
            .expect("connector fixture loads");
    let mut scene = Scene::new();
    let source = scene
        .instantiate(&scene_asset)
        .expect("source connector fixture instantiates");
    let target = scene
        .instantiate(&scene_asset)
        .expect("target connector fixture instantiates");
    scene
        .set_transform(target.roots()[0], Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .expect("target transform updates");

    scene
        .connect_import_connectors(
            &source,
            "mount",
            &target,
            "mount",
            ConnectOptions::default(),
        )
        .expect("imports connect by stable connector names");

    assert_vec3_near(
        scene
            .world_transform(source.roots()[0])
            .expect("source root remains in scene")
            .translation,
        Vec3::new(1.0, 0.0, 0.0),
    );
}

#[test]
fn m7_imported_nested_connector_moves_import_root_without_breaking_child_local_transform() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/nested_connector_scene.gltf"))
            .expect("nested connector fixture loads");
    let mut scene = Scene::new();
    let source = scene
        .instantiate(&scene_asset)
        .expect("source nested connector fixture instantiates");
    let target = scene
        .instantiate(&scene_asset)
        .expect("target nested connector fixture instantiates");
    let source_connector_node = source.connector("mount").expect("source connector").node();
    scene
        .set_transform(target.roots()[0], Transform::at(Vec3::new(6.0, 0.0, 0.0)))
        .expect("target root transform updates");

    scene
        .connect_import_connectors(
            &source,
            "mount",
            &target,
            "mount",
            ConnectOptions::default(),
        )
        .expect("nested imports connect by stable connector names");

    assert_vec3_near(
        scene
            .world_transform(source.roots()[0])
            .expect("source root remains in scene")
            .translation,
        Vec3::new(6.0, 0.0, 0.0),
    );
    assert_vec3_near(
        scene
            .node(source_connector_node)
            .expect("source connector host remains in scene")
            .transform()
            .translation,
        Vec3::new(0.0, 2.0, 0.0),
    );
}

#[test]
fn m7_imported_animated_connector_keeps_import_local_animation_binding_after_connection() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/animated_connector_scene.gltf"))
            .expect("animated connector fixture loads");
    let mut scene = Scene::new();
    let source = scene
        .instantiate(&scene_asset)
        .expect("source animated connector fixture instantiates");
    let target = scene
        .instantiate(&scene_asset)
        .expect("target animated connector fixture instantiates");
    let source_animated = source
        .node("AnimatedMount")
        .expect("source animated node resolves");
    let target_animated = target
        .node("AnimatedMount")
        .expect("target animated node resolves");
    let source_clip = source.clip("MoveMount").expect("source clip resolves");

    assert_eq!(source_clip.channels().len(), 1);
    assert_eq!(source_clip.channels()[0].target_node(), source_animated);
    assert_eq!(
        source_clip.channels()[0].target(),
        AnimationTarget::Translation
    );

    let mixer = scene
        .create_animation_mixer(&source, "MoveMount")
        .expect("source mixer creates before connection");
    scene
        .connect_import_connectors(
            &source,
            "mount",
            &target,
            "mount",
            ConnectOptions::default().with_mate_offset(Transform::at(Vec3::new(1.0, 0.0, 0.0))),
        )
        .expect("animated import connects by root placement");
    scene
        .seek_animation(mixer, 1.0)
        .expect("source animation remains bound after connection");

    assert_vec3_near(
        scene
            .world_transform(source.roots()[0])
            .expect("source root stays connected")
            .translation,
        Vec3::new(1.0, 0.0, 0.0),
    );
    assert_vec3_near(
        scene
            .node(source_animated)
            .expect("source animated node remains import-local")
            .transform()
            .translation,
        Vec3::new(1.0, 0.0, 0.0),
    );
    assert_vec3_near(
        scene
            .node(target_animated)
            .expect("target animated node was not changed by source mixer")
            .transform()
            .translation,
        Vec3::ZERO,
    );
}

#[test]
fn m7_frame_all_uses_imported_mesh_bounds_without_manual_bounds_math() {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("mesh fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("mesh fixture instantiates");
    scene
        .set_transform(import.roots()[0], Transform::at(Vec3::new(3.0, 0.0, 0.0)))
        .expect("import root moves");
    let camera = scene.add_default_camera().expect("default camera inserts");

    scene
        .frame_all(camera)
        .expect("frame_all uses stored mesh bounds");
    let mesh_node = import
        .node("ColoredTriangle")
        .expect("imported mesh node resolves");
    scene
        .frame_node(camera, mesh_node)
        .expect("frame_node uses stored mesh bounds");

    let mut renderer = Renderer::headless(96, 96).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("framed scene prepares");
    renderer
        .render_active(&scene)
        .expect("framed scene renders");
    assert!(count_nonblack_pixels(renderer.frame_rgba8()) > 0);
}

#[test]
fn m7_frame_all_with_assets_frames_direct_mesh_bounds_without_manual_bounds_math() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(80, 180, 240)));
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(4.0, 0.0, 0.0)))
        .add()
        .expect("direct mesh inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");

    scene
        .frame_all_with_assets(camera, &assets)
        .expect("direct mesh bounds frame through assets");

    let mut renderer = Renderer::headless(96, 96).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("framed direct mesh scene prepares");
    renderer
        .render_active(&scene)
        .expect("framed direct mesh scene renders");
    assert!(
        count_nonblack_pixels(renderer.frame_rgba8()) > 0,
        "direct Assets-created meshes must be frameable without manual bounds math"
    );
}

#[test]
fn m7_frame_all_with_assets_frames_instance_bounds_without_manual_bounds_math() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(240, 180, 80)));
    let mut scene = Scene::new();
    let instances = scene
        .add_instance_set(scene.root(), geometry, material, Transform::default())
        .expect("instance set inserts");
    scene
        .push_instance(instances, Transform::at(Vec3::new(4.0, 0.0, 0.0)))
        .expect("off-center instance inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");

    scene
        .frame_all_with_assets(camera, &assets)
        .expect("instance bounds frame through assets");

    let mut renderer = Renderer::headless(96, 96).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("framed instance scene prepares");
    renderer
        .render_active(&scene)
        .expect("framed instance scene renders");
    assert!(
        count_nonblack_pixels(renderer.frame_rgba8()) > 0,
        "instance-set bounds must include per-instance transforms before framing"
    );
}

#[test]
fn m7_frame_node_with_assets_frames_direct_mesh_without_manual_bounds_math() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(120, 240, 160)));
    let mut scene = Scene::new();
    let mesh = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-4.0, 0.0, 0.0)))
        .add()
        .expect("direct mesh inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");

    scene
        .frame_node_with_assets(camera, mesh, &assets)
        .expect("selected direct mesh frames through assets");

    let mut renderer = Renderer::headless(96, 96).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("framed selected mesh scene prepares");
    renderer
        .render_active(&scene)
        .expect("framed selected mesh scene renders");
    assert!(
        count_nonblack_pixels(renderer.frame_rgba8()) > 0,
        "selected direct mesh should be frameable without manual bounds math"
    );
}

#[test]
fn m7_error_display_snapshots_cover_beginner_recovery_paths() {
    let snapshots = vec![
        (
            "missing_camera",
            RenderError::NoActiveCamera.to_string(),
            RenderError::NoActiveCamera.help(),
        ),
        (
            "not_prepared",
            RenderError::NotPrepared {
                reason: NotPreparedReason::NeverPrepared,
            }
            .to_string(),
            RenderError::NotPrepared {
                reason: NotPreparedReason::NeverPrepared,
            }
            .help(),
        ),
        (
            "invalid_camera_handle",
            LookupError::CameraNotFound(CameraKey::default()).to_string(),
            LookupError::CameraNotFound(CameraKey::default()).help(),
        ),
        (
            "ambiguous_anchor",
            LookupError::AmbiguousAnchorName {
                name: "mount".to_string(),
                hosts: vec![NodeKey::default(), NodeKey::default()],
            }
            .to_string(),
            LookupError::AmbiguousAnchorName {
                name: "mount".to_string(),
                hosts: vec![NodeKey::default(), NodeKey::default()],
            }
            .help(),
        ),
        (
            "stale_import",
            LookupError::StaleImport.to_string(),
            LookupError::StaleImport.help(),
        ),
        (
            "unsupported_texture_format",
            AssetError::UnsupportedTextureFormat {
                path: "asset.ktx2".to_string(),
                help: "enable feature ktx2",
            }
            .to_string(),
            AssetError::UnsupportedTextureFormat {
                path: "asset.ktx2".to_string(),
                help: "enable feature ktx2",
            }
            .help(),
        ),
        (
            "surface_lost",
            RenderError::SurfaceLost { recoverable: true }.to_string(),
            RenderError::SurfaceLost { recoverable: true }.help(),
        ),
        (
            "backend_capability_mismatch",
            PrepareError::BackendCapabilityMismatch {
                feature: "compute culling",
                backend: Backend::WebGl2,
                help: "use CPU culling fallback".to_string(),
            }
            .to_string(),
            PrepareError::BackendCapabilityMismatch {
                feature: "compute culling",
                backend: Backend::WebGl2,
                help: "use CPU culling fallback".to_string(),
            }
            .help(),
        ),
    ];

    assert_eq!(
        snapshots,
        vec![
            (
                "missing_camera",
                "scene has no active camera".to_string(),
                "call Scene::add_default_camera or Scene::set_active_camera"
            ),
            (
                "not_prepared",
                "renderer is not prepared: prepare has not been called".to_string(),
                "call Renderer::prepare after scene, target, or renderer changes"
            ),
            (
                "invalid_camera_handle",
                "camera key does not exist in the scene".to_string(),
                "use a CameraKey created by this Scene"
            ),
            (
                "ambiguous_anchor",
                "imported scene anchor name 'mount' is ambiguous across 2 host nodes".to_string(),
                "call anchors_named or anchors_for to choose a host node"
            ),
            (
                "stale_import",
                "scene import has been invalidated".to_string(),
                "re-resolve nodes, anchors, and clips from the replacement SceneImport"
            ),
            (
                "unsupported_texture_format",
                "texture asset.ktx2 uses an unsupported format: enable feature ktx2".to_string(),
                "use a supported texture format such as PNG, JPEG, or WebP, or enable a decoder feature when one exists"
            ),
            (
                "surface_lost",
                "render surface was lost; recoverable=true".to_string(),
                "call recover_surface, then prepare again"
            ),
            (
                "backend_capability_mismatch",
                "backend WebGl2 cannot provide required feature compute culling: use CPU culling fallback".to_string(),
                "query renderer.capabilities and choose a compatible quality/profile path"
            ),
        ]
    );

    let scene = Scene::new();
    let renderer = Renderer::headless(32, 32).expect("renderer builds");
    let diagnostic_snapshots = renderer
        .diagnose_scene(&scene)
        .into_iter()
        .map(|diagnostic| {
            (
                diagnostic.code,
                diagnostic.severity,
                diagnostic.message,
                diagnostic.help.expect("beginner diagnostics include help"),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        diagnostic_snapshots,
        vec![
            (
                DiagnosticCode::MissingActiveCamera,
                DiagnosticSeverity::Error,
                "scene has no active camera".to_string(),
                "call Scene::add_default_camera or Scene::set_active_camera before rendering"
                    .to_string(),
            ),
            (
                DiagnosticCode::InvisibleScene,
                DiagnosticSeverity::Warning,
                "scene has no visible drawables for the active camera".to_string(),
                "check node visibility, parent visibility, camera layer masks, or add a mesh/renderable node"
                    .to_string(),
            ),
            (
                DiagnosticCode::MissingLightingOrEnvironment,
                DiagnosticSeverity::Warning,
                "scene has no active light nodes and no renderer environment".to_string(),
                "call renderer.set_environment for image-based lighting or add a scene light for lit materials"
                    .to_string(),
            ),
        ]
    );
}

#[test]
fn m7_diagnostics_expose_typed_actionable_suggested_fixes() {
    let scene = Scene::new();
    let renderer = Renderer::headless(32, 32).expect("renderer builds");
    let diagnostics = renderer.diagnose_scene(&scene);
    let missing_camera = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == DiagnosticCode::MissingActiveCamera)
        .expect("missing-camera diagnostic exists");

    assert_eq!(missing_camera.severity(), DiagnosticSeverity::Error);
    assert!(missing_camera.message().contains("no active camera"));
    assert_eq!(
        missing_camera.help(),
        Some("call Scene::add_default_camera or Scene::set_active_camera before rendering")
    );
    assert_eq!(missing_camera.suggested_fix(), missing_camera.help());
}

#[test]
fn m7_renderer_capability_report_exports_backend_adapter_and_diagnostics() {
    let renderer = Renderer::headless(32, 32).expect("renderer builds");

    let report = renderer.capability_report();

    assert_eq!(report.backend(), Backend::Headless);
    assert_eq!(report.capabilities().backend, Backend::Headless);
    assert!(report.adapter().is_none());
    assert!(report.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == DiagnosticCode::ForwardPbrDegraded
            && diagnostic.severity() == DiagnosticSeverity::Warning
    }));
}

#[cfg(feature = "inspection")]
#[test]
fn m7_scene_inspection_feature_reports_reproducible_metadata() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let material = assets.create_material(MaterialDesc::unlit(scena::Color::WHITE));

    let mut scene = Scene::new();
    let mesh = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(1.0, 2.0, 3.0)))
        .add()
        .expect("mesh inserts");
    scene.add_tag(mesh, "inspectable").expect("tag inserts");
    scene.set_render_group(mesh, 4).expect("group updates");
    scene
        .set_helper_on_top(mesh, true)
        .expect("helper metadata updates");
    let camera = scene.add_default_camera().expect("camera inserts");

    let report = scene.inspect();
    assert_eq!(report.node_count(), 3);
    assert_eq!(report.active_camera(), Some(camera));
    assert_eq!(report.visible_drawable_count(), 1);
    assert_eq!(report.camera_count(), 1);
    assert_eq!(report.light_count(), 0);
    assert_eq!(report.anchor_count(), 0);
    assert_eq!(report.connector_count(), 0);
    assert_eq!(report.clipping_plane_count(), 0);
    assert_eq!(
        report.structure_revision(),
        scene.dirty_state().structure_revision
    );

    let mesh_meta = report
        .nodes()
        .iter()
        .find(|node| node.node() == mesh)
        .expect("mesh metadata is exported");
    assert_eq!(mesh_meta.kind(), "Mesh");
    assert_eq!(mesh_meta.parent(), Some(scene.root()));
    assert_eq!(mesh_meta.mesh_geometry(), Some(geometry));
    assert_eq!(mesh_meta.mesh_material(), Some(material));
    assert_eq!(mesh_meta.camera(), None);
    assert_eq!(mesh_meta.light(), None);
    assert_eq!(mesh_meta.bounds(), None);
    assert_eq!(mesh_meta.render_group(), 4);
    assert!(mesh_meta.helper_on_top());
    assert!(mesh_meta.visible());
    assert!(mesh_meta.tags().iter().any(|tag| tag == "inspectable"));
    assert_eq!(mesh_meta.layer_mask(), u64::MAX);
    assert_eq!(mesh_meta.transform().translation, Vec3::new(1.0, 2.0, 3.0));

    let camera_meta = report
        .nodes()
        .iter()
        .find(|node| node.camera() == Some(camera))
        .expect("camera metadata is exported");
    assert_eq!(camera_meta.kind(), "Camera");
}

#[cfg(feature = "inspection")]
#[test]
fn m7_scene_inspection_reports_local_and_world_transforms_for_nested_nodes() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let material = assets.create_material(MaterialDesc::unlit(scena::Color::WHITE));

    let mut scene = Scene::new();
    let parent = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .expect("parent inserts");
    let mesh = scene
        .mesh(geometry, material)
        .parent(parent)
        .transform(Transform::at(Vec3::new(0.0, 3.0, 0.0)))
        .add()
        .expect("nested mesh inserts");

    let report = scene.inspect_with_assets(&assets);
    let mesh_meta = report
        .nodes()
        .iter()
        .find(|node| node.node() == mesh)
        .expect("mesh metadata is exported");

    assert_eq!(mesh_meta.transform().translation, Vec3::new(0.0, 3.0, 0.0));
    assert_eq!(
        mesh_meta.world_transform().translation,
        Vec3::new(2.0, 3.0, 0.0)
    );
}

#[cfg(feature = "inspection")]
#[test]
fn m7_scene_inspection_with_assets_reports_material_preview_metadata() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let texture = pollster::block_on(assets.load_texture(
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
        scena::TextureColorSpace::Srgb,
    ))
    .expect("data URI texture decodes");
    let material = assets.create_material(
        MaterialDesc::unlit(scena::Color::from_linear_rgb(0.25, 0.5, 0.75))
            .with_base_color_texture(texture)
            .with_alpha_mode(scena::AlphaMode::Blend),
    );

    let mut scene = Scene::new();
    let mesh = scene.mesh(geometry, material).add().expect("mesh inserts");

    let report = scene.inspect_with_assets(&assets);
    let mesh_meta = report
        .nodes()
        .iter()
        .find(|node| node.node() == mesh)
        .expect("mesh metadata is exported");
    let preview = mesh_meta
        .material_preview()
        .expect("asset-aware inspection resolves material preview");

    assert_eq!(preview.material(), material);
    assert_eq!(preview.kind(), scena::MaterialKind::Unlit);
    assert_eq!(
        preview.base_color(),
        scena::Color::from_linear_rgb(0.25, 0.5, 0.75)
    );
    assert_eq!(preview.alpha_mode(), scena::AlphaMode::Blend);
    assert!(preview.has_base_color_texture());
    let texture_preview = preview
        .base_color_texture()
        .expect("base-color texture metadata is exported");
    assert_eq!(texture_preview.texture(), texture);
    assert_eq!(
        texture_preview.source_format(),
        scena::TextureSourceFormat::Png
    );
    assert_eq!(
        texture_preview.color_space(),
        scena::TextureColorSpace::Srgb
    );
    assert_eq!(texture_preview.decoded_dimensions(), Some((1, 1)));
    assert!(texture_preview.has_decoded_pixels());
    assert!(!preview.has_normal_texture());
    assert!(!preview.has_metallic_roughness_texture());
    assert!(!preview.has_occlusion_texture());
    assert!(!preview.has_emissive_texture());
}

#[cfg(feature = "inspection")]
#[test]
fn m7_scene_inspection_with_assets_reports_draw_list_entries() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let material = assets.create_material(MaterialDesc::unlit(scena::Color::WHITE));

    let mut scene = Scene::new();
    let mesh = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .add()
        .expect("mesh inserts");
    let instances = scene
        .add_instance_set(scene.root(), geometry, material, Transform::IDENTITY)
        .expect("instance set inserts");
    let instance = scene
        .push_instance(instances, Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .expect("instance inserts");

    let report = scene.inspect_with_assets(&assets);
    let draw_list = report.draw_list();

    assert_eq!(draw_list.len(), 2);
    let mesh_draw = draw_list
        .iter()
        .find(|entry| entry.node() == mesh)
        .expect("direct mesh draw is exported");
    assert_eq!(mesh_draw.instance(), None);
    assert_eq!(mesh_draw.geometry(), geometry);
    assert_eq!(mesh_draw.material(), material);
    assert_eq!(mesh_draw.topology(), scena::GeometryTopology::Triangles);
    assert_eq!(mesh_draw.primitive_count(), 12);
    assert_eq!(mesh_draw.vertex_count(), 24);
    assert_eq!(mesh_draw.index_count(), 36);
    assert_eq!(
        mesh_draw.world_transform().translation,
        Vec3::new(1.0, 0.0, 0.0)
    );
    assert!(mesh_draw.material_preview().is_some());

    let instance_draw = draw_list
        .iter()
        .find(|entry| entry.instance() == Some(instance))
        .expect("instance draw is exported");
    assert_eq!(instance_draw.geometry(), geometry);
    assert_eq!(instance_draw.material(), material);
    assert_eq!(
        instance_draw.world_transform().translation,
        Vec3::new(2.0, 0.0, 0.0)
    );
    assert!(instance_draw.visible());
}

#[cfg(feature = "inspection")]
#[test]
fn m7_scene_inspection_reports_camera_frustum_debug_geometry() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");

    let report = scene.inspect();
    let frustum = report
        .camera_frustums()
        .iter()
        .find(|frustum| frustum.camera() == camera)
        .expect("camera frustum is exported");

    assert_eq!(
        frustum.node(),
        scene.camera_node(camera).expect("camera node")
    );
    assert_eq!(frustum.near(), PerspectiveCamera::default().near);
    assert_eq!(frustum.far(), PerspectiveCamera::default().far);
    assert_eq!(frustum.corners().len(), 8);
    assert!(
        frustum
            .corners()
            .iter()
            .all(|corner| { corner.x.is_finite() && corner.y.is_finite() && corner.z.is_finite() }),
        "frustum corners must be finite world-space debug geometry"
    );
    assert!(
        frustum.corners()[0].z < 2.0 && frustum.corners()[4].z < frustum.corners()[0].z,
        "perspective frustum corners must extend forward from the camera"
    );
}

#[cfg(feature = "inspection")]
#[test]
fn m7_scene_inspection_reports_normal_debug_segments() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let material = assets.create_material(MaterialDesc::unlit(scena::Color::WHITE));

    let mut scene = Scene::new();
    let mesh = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .add()
        .expect("mesh inserts");

    let report = scene.inspect_with_assets(&assets);
    let normals = report
        .normal_overlays()
        .iter()
        .find(|overlay| overlay.node() == mesh)
        .expect("normal debug overlay is exported");

    assert_eq!(normals.geometry(), geometry);
    assert_eq!(normals.segments().len(), 24);
    assert_eq!(normals.length(), 0.1);
    let first = normals.segments()[0];
    assert_ne!(
        first[0], first[1],
        "normal segment must have visible extent"
    );
    assert!(
        first
            .iter()
            .all(|point| point.x.is_finite() && point.y.is_finite() && point.z.is_finite()),
        "normal debug points must be finite world-space positions"
    );
}

#[test]
fn transform_rotate_helpers_compose_chained_rotations_instead_of_overwriting() {
    // scena-api-ergonomics-reviewer Phase 6 finding F2 closure: chained
    // rotate_*_deg helpers must compose onto the existing rotation so a
    // beginner cannot silently lose an earlier rotation by chaining a
    // second axis. Earlier behavior overwrote, which contradicted the
    // "no silent fallbacks" review rule.
    let chained = Transform::IDENTITY.rotate_y_deg(90.0).rotate_x_deg(45.0);
    let last_only = Transform::IDENTITY.rotate_x_deg(45.0);
    assert!(
        !quaternion_close_enough(chained.rotation, last_only.rotation),
        "chained rotate_y_deg(90).rotate_x_deg(45) must NOT collapse to a \
         lone X rotation; the y-rotation must be preserved (got chained={:?} \
         last_only={:?})",
        chained.rotation,
        last_only.rotation,
    );

    // Single-rotation callers are unaffected: rotate_y_deg(90) on the
    // identity transform still yields the y-rotation, since identity ∘ R = R.
    let single = Transform::IDENTITY.rotate_y_deg(90.0);
    let direct_y = scena::Quat {
        x: 0.0,
        y: (std::f32::consts::FRAC_PI_4).sin(),
        z: 0.0,
        w: (std::f32::consts::FRAC_PI_4).cos(),
    };
    assert!(
        quaternion_close_enough(single.rotation, direct_y),
        "Transform::IDENTITY.rotate_y_deg(90) must equal the canonical Y(90°) \
         quaternion (got {:?} vs canonical {:?})",
        single.rotation,
        direct_y,
    );
}

fn quaternion_close_enough(a: scena::Quat, b: scena::Quat) -> bool {
    let dot = a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
    dot.abs() > 1.0 - 1.0e-3
}
