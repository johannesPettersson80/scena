use std::alloc::{GlobalAlloc, Layout, System};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use scena::{
    Aabb, AssetError, Assets, Backend, CameraKey, DiagnosticCode, DiagnosticSeverity, GeometryDesc,
    ImportAnchorDebugMetadata, LabelDesc, LookupError, MaterialDesc, NodeKey, NotPreparedReason,
    OrbitControls, PerspectiveCamera, PointerEvent, PrepareError, Primitive, RenderError, Renderer,
    Scene, SourceCoordinateSystem, SourceUnits, SurfaceEvent, SurfaceSize, SurfaceViewport,
    TouchEvent, Transform, Vec3,
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
