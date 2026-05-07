use scena::{
    Aabb, Assets, DiagnosticCode, DiagnosticSeverity, GeometryDesc, ImportAnchorDebugMetadata,
    LookupError, MaterialDesc, OrbitControls, PerspectiveCamera, PointerEvent, Primitive,
    RenderError, Renderer, Scene, SourceCoordinateSystem, SourceUnits, SurfaceEvent, SurfaceSize,
    SurfaceViewport, TouchEvent, Transform, Vec3,
};

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
    assert_eq!(mesh_meta.render_group(), 4);
    assert!(mesh_meta.helper_on_top());
    assert!(mesh_meta.visible());
    assert!(mesh_meta.tags().iter().any(|tag| tag == "inspectable"));
    assert_eq!(mesh_meta.layer_mask(), u64::MAX);
    assert_eq!(mesh_meta.transform().translation, Vec3::new(1.0, 2.0, 3.0));
}
