#![cfg(not(target_arch = "wasm32"))]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use scena::{
    Assets, Backend, CameraKey, CapabilityStatus, Color, GeometryDesc, GeometryTopology,
    GeometryVertex, HardwareTier, MaterialDesc, OrbitControlAction, OrbitControls, PlatformSurface,
    PointerButton, PointerEvent, PointerEventKind, Profile, Quality, RenderError, RenderMode,
    Renderer, RendererOptions, Scene, SurfaceEvent, Transform, Vec3,
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
        // SAFETY: this allocator only observes allocation counts and delegates the actual
        // allocation to the standard system allocator with the original layout.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the pointer and layout are forwarded unchanged to the allocator that
        // created the allocation.
        unsafe { System.dealloc(pointer, layout) }
    }
}

fn scene_with_camera() -> (Scene, CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            scena::PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts under root");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");
    scene
        .add_renderable(
            scene.root(),
            vec![scena::Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("triangle inserts under root");
    (scene, camera)
}

#[test]
fn capability_matrix_reports_hardware_tier_and_backend_feature_states() {
    let headless = *Renderer::headless(16, 16)
        .expect("headless renderer builds")
        .capabilities();
    assert_eq!(headless.backend, Backend::Headless);
    assert_eq!(headless.hardware_tier, HardwareTier::Low);
    assert_eq!(headless.forward_pbr, CapabilityStatus::Degraded);
    assert_eq!(
        headless.directional_shadows,
        CapabilityStatus::Degraded,
        "shadow-map allocation metadata is not visible shadow rendering proof"
    );
    assert_eq!(
        headless.point_shadows,
        CapabilityStatus::FeatureDisabled,
        "point-light shadow maps are not implemented and must not be implied by light support"
    );
    assert_eq!(
        headless.spot_shadows,
        CapabilityStatus::FeatureDisabled,
        "spot-light shadow maps are not implemented and must not be implied by light support"
    );
    assert_eq!(
        headless.bloom,
        CapabilityStatus::FeatureDisabled,
        "bloom is not implemented and must not be implied by the FXAA/output stage"
    );
    assert_eq!(
        headless.screen_space_ambient_occlusion,
        CapabilityStatus::FeatureDisabled,
        "SSAO/GTAO is not implemented and must not be implied by depth support"
    );
    assert_eq!(
        headless.gpu_frustum_culling,
        CapabilityStatus::FeatureDisabled
    );
    assert_eq!(
        headless.per_instance_culling,
        CapabilityStatus::Degraded,
        "CPU culling is the documented fallback for non-compute lanes"
    );
    assert_eq!(headless.compute_shaders, CapabilityStatus::FeatureDisabled);
    // Phase 1F: CPU rasterizer never samples array textures; the field
    // reports FeatureDisabled with zero layers so cap-matrix consumers can
    // distinguish CPU lanes from GPU lanes that meet the WebGPU minimum.
    assert_eq!(headless.texture_arrays, CapabilityStatus::FeatureDisabled);
    assert_eq!(headless.max_texture_array_layers, 0);

    let webgl2 = scena::Capabilities::for_backend(Backend::WebGl2);
    assert_eq!(webgl2.hardware_tier, HardwareTier::Low);
    assert_eq!(webgl2.max_clipping_planes, 8);
    assert_eq!(webgl2.default_clipping_planes, 4);
    assert_eq!(webgl2.ibl_cubemap_default_size, 128);
    assert_eq!(
        webgl2.texture_compression_basisu,
        CapabilityStatus::FeatureDisabled
    );
    assert_eq!(
        webgl2.hardware_instancing,
        CapabilityStatus::FeatureDisabled
    );
    // Phase 1F: WebGL2 GLES 3.0+ exposes sampler2DArray; renderer can pack
    // per-role textures into a single array texture once Phase 1F step 2
    // lands the actual batching impl.
    assert_eq!(webgl2.texture_arrays, CapabilityStatus::Supported);
    assert_eq!(webgl2.max_texture_array_layers, 256);
    assert_eq!(
        webgl2.fragment_high_precision,
        CapabilityStatus::FeatureDisabled
    );
    assert_eq!(webgl2.uniform_buffers, CapabilityStatus::FeatureDisabled);
    assert_eq!(webgl2.uniform_buffer_max_bytes, 16_384);
    assert_eq!(
        webgl2.gpu_frustum_culling,
        CapabilityStatus::FeatureDisabled
    );
    assert_eq!(webgl2.per_instance_culling, CapabilityStatus::Degraded);
    assert_eq!(webgl2.storage_buffers, CapabilityStatus::FeatureDisabled);

    let webgpu = scena::Capabilities::for_attached_gpu_backend(Backend::WebGpu);
    assert_eq!(webgpu.hardware_tier, HardwareTier::Medium);
    assert_eq!(webgpu.forward_pbr, CapabilityStatus::Degraded);
    assert_eq!(
        webgpu.gpu_frustum_culling,
        CapabilityStatus::FeatureDisabled
    );
    assert_eq!(webgpu.per_instance_culling, CapabilityStatus::Supported);
    assert_eq!(webgpu.compute_shaders, CapabilityStatus::Supported);
    assert_eq!(webgpu.texture_arrays, CapabilityStatus::Supported);
    assert_eq!(webgpu.max_texture_array_layers, 256);

    let diagnostics = webgpu.diagnostics();
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == scena::DiagnosticCode::ForwardPbrDegraded
            && diagnostic.message.contains("PBR")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == scena::DiagnosticCode::DirectionalShadowsDegraded
            && diagnostic.message.contains("Directional shadows")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == scena::DiagnosticCode::PointShadowsDisabled
            && diagnostic.message.contains("Point shadows")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == scena::DiagnosticCode::SpotShadowsDisabled
            && diagnostic.message.contains("Spot shadows")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == scena::DiagnosticCode::BloomDisabled
            && diagnostic.message.contains("Bloom")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == scena::DiagnosticCode::AmbientOcclusionDisabled
            && diagnostic.message.contains("ambient occlusion")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == scena::DiagnosticCode::GpuCullingDisabled
            && diagnostic.message.contains("GPU culling")
    }));
}

#[test]
fn renderer_options_apply_profile_quality_and_render_mode_precedence() {
    let renderer = Renderer::headless_with_options(
        16,
        16,
        RendererOptions::default()
            .with_profile(Profile::Compatibility)
            .with_quality(Quality::High)
            .with_render_mode(RenderMode::OnChange),
    )
    .expect("renderer builds");

    assert_eq!(renderer.profile(), Profile::Compatibility);
    assert_eq!(
        renderer.quality(),
        Quality::High,
        "explicit quality overrides profile and hardware defaults"
    );
    assert_eq!(renderer.render_mode(), RenderMode::OnChange);
}

#[test]
fn on_change_render_static_idle_records_skipped_frame_stats() {
    let (mut scene, camera) = scene_with_camera();
    let mut renderer = Renderer::headless_with_options(
        32,
        32,
        RendererOptions::default().with_render_mode(RenderMode::OnChange),
    )
    .expect("renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");

    let first = renderer.render(&scene, camera).expect("first render draws");
    let second = renderer
        .render(&scene, camera)
        .expect("unchanged on-change render is skipped");

    assert!(!first.skipped);
    assert!(second.skipped);
    assert_eq!(second.draw_calls, 0);
    assert_eq!(renderer.stats().frames_rendered, 1);
    assert_eq!(renderer.stats().skipped_frames, 1);
}

#[test]
fn render_on_change_static_idle_skip_has_zero_allocations() {
    let (mut scene, camera) = scene_with_camera();
    let mut renderer = Renderer::headless_with_options(
        32,
        32,
        RendererOptions::default().with_render_mode(RenderMode::OnChange),
    )
    .expect("renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("warm-up render draws");

    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
    COUNT_ALLOCATIONS.store(true, Ordering::Relaxed);
    let outcome = renderer
        .render(&scene, camera)
        .expect("static idle frame is skipped");
    COUNT_ALLOCATIONS.store(false, Ordering::Relaxed);

    assert!(outcome.skipped);
    assert_eq!(ALLOCATION_COUNT.load(Ordering::Relaxed), 0);
}

#[test]
fn transform_dirty_state_propagates_through_world_transform_queries() {
    let mut scene = Scene::new();
    let parent = scene
        .add_empty(scene.root(), Transform::default())
        .expect("parent inserts");
    let child = scene
        .add_empty(
            parent,
            Transform {
                translation: Vec3::new(1.0, 0.0, 0.0),
                ..Transform::default()
            },
        )
        .expect("child inserts");
    let before = scene.dirty_state();

    scene
        .set_transform(
            parent,
            Transform {
                translation: Vec3::new(2.0, 0.0, 0.0),
                ..Transform::default()
            },
        )
        .expect("parent transform changes");
    let after = scene.dirty_state();
    let child_world = scene
        .world_transform(child)
        .expect("child world transform resolves through parent");

    assert!(after.transform_revision > before.transform_revision);
    assert_eq!(child_world.translation, Vec3::new(3.0, 0.0, 0.0));
}

#[test]
fn cpu_frustum_culling_drops_offscreen_renderables_before_draw() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            scena::PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");
    scene
        .add_renderable(
            scene.root(),
            vec![scena::Primitive::unlit_triangle()],
            Transform {
                translation: Vec3::new(3.0, 0.0, 0.0),
                ..Transform::default()
            },
        )
        .expect("offscreen triangle inserts");
    let mut renderer = Renderer::headless(32, 32).expect("renderer builds");

    renderer.prepare(&mut scene).expect("prepare succeeds");
    let outcome = renderer.render(&scene, camera).expect("render succeeds");

    assert_eq!(outcome.draw_calls, 0);
    assert_eq!(renderer.stats().culled_objects, 1);
    assert!(
        renderer
            .frame_rgba8()
            .chunks_exact(4)
            .all(|pixel| pixel[0..3] == [0, 0, 0])
    );
}

#[test]
fn per_instance_cpu_culling_keeps_visible_instances_and_counts_culled_ones() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            scena::PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");
    let instances = scene
        .add_instance_set(scene.root(), geometry, material, Transform::default())
        .expect("instance set inserts");
    scene
        .push_instance(instances, Transform::default())
        .expect("visible instance inserts");
    scene
        .push_instance(
            instances,
            Transform {
                translation: Vec3::new(4.0, 0.0, 0.0),
                ..Transform::default()
            },
        )
        .expect("offscreen instance inserts");
    let mut renderer = Renderer::headless(32, 32).expect("renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("prepare with instances succeeds");
    let outcome = renderer.render(&scene, camera).expect("render succeeds");

    assert_eq!(outcome.draw_calls, 1);
    assert_eq!(renderer.stats().culled_objects, 1);
}

#[test]
fn gpu_capable_renderer_records_compute_culling_dispatch_when_available() {
    match Renderer::headless_gpu(32, 32) {
        Ok(mut renderer) => {
            let (mut scene, camera) = scene_with_camera();
            renderer.prepare(&mut scene).expect("prepare succeeds");
            renderer.render(&scene, camera).expect("render succeeds");

            assert_eq!(
                renderer.capabilities().gpu_frustum_culling,
                CapabilityStatus::FeatureDisabled
            );
            assert_eq!(renderer.stats().gpu_culling_dispatches, 0);
        }
        Err(scena::BuildError::NoAdapter { backend })
        | Err(scena::BuildError::RequestDevice { backend }) => {
            assert_eq!(backend, Backend::HeadlessGpu);
        }
        Err(error) => panic!("unexpected headless GPU setup result: {error:?}"),
    }
}

#[test]
fn surface_loss_requires_recovery_and_prepare_before_render() {
    let (mut scene, camera) = scene_with_camera();
    let mut renderer = Renderer::from_surface(PlatformSurface::native_window(32, 32))
        .expect("surface descriptor renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");
    renderer.render(&scene, camera).expect("render succeeds");

    renderer
        .handle_surface_event(SurfaceEvent::Lost)
        .expect("surface loss event is recorded");
    assert_eq!(
        renderer.render(&scene, camera),
        Err(RenderError::SurfaceLost { recoverable: true })
    );

    renderer
        .recover_surface(PlatformSurface::native_window(64, 64))
        .expect("descriptor surface recovers");
    assert!(matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared { .. })
    ));

    renderer
        .prepare(&mut scene)
        .expect("prepare after recovery succeeds");
    let outcome = renderer.render(&scene, camera).expect("render recovers");
    assert_eq!(outcome.width, 64);
    assert_eq!(outcome.height, 64);
}

fn fullscreen_triangle_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            GeometryVertex {
                position: Vec3::new(-0.5, -0.5, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(0.5, -0.5, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            GeometryVertex {
                position: Vec3::new(0.0, 0.5, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2],
    )
    .expect("triangle geometry is valid")
}

#[test]
fn dpr_change_marks_surface_state_dirty_until_prepare() {
    let (mut scene, camera) = scene_with_camera();
    let mut renderer = Renderer::from_surface(PlatformSurface::native_window(32, 32))
        .expect("surface descriptor renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");
    renderer.render(&scene, camera).expect("render succeeds");

    renderer
        .handle_surface_event(SurfaceEvent::ScaleFactorChanged { scale_factor: 2.0 })
        .expect("DPR event is accepted");

    assert!(matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared { .. })
    ));
}

#[test]
fn context_recovery_requires_retained_assets_and_reprepare() {
    let (mut scene, camera) = scene_with_camera();
    let assets = Assets::new();
    let mut renderer = Renderer::headless(32, 32).expect("renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");
    renderer.render(&scene, camera).expect("render succeeds");

    renderer
        .handle_surface_event(SurfaceEvent::ContextLost { recoverable: true })
        .expect("context loss event is recorded");
    assert_eq!(
        renderer.render(&scene, camera),
        Err(RenderError::ContextLost { recoverable: true })
    );

    renderer
        .handle_surface_event(SurfaceEvent::ContextRestored)
        .expect("context restoration event is recorded");
    renderer
        .recover_context(&assets, &mut scene)
        .expect("headless retained context recovery succeeds");
    assert!(matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared { .. })
    ));
}

#[test]
fn context_recovery_rejects_assets_without_retained_cpu_data() {
    let (mut scene, _camera) = scene_with_camera();
    let mut assets = Assets::new();
    assets.set_retain_policy(scena::RetainPolicy::Never);
    let mut renderer = Renderer::headless(32, 32).expect("renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");
    renderer
        .handle_surface_event(SurfaceEvent::ContextLost { recoverable: true })
        .expect("context loss event is recorded");

    let error = renderer
        .recover_context(&assets, &mut scene)
        .expect_err("context recovery needs retained CPU-side asset data");

    assert!(matches!(
        error,
        scena::PrepareError::BackendCapabilityMismatch {
            feature: "context recovery",
            ..
        }
    ));
}

#[test]
fn public_threading_contract_is_statically_enforced() {
    fn assert_send<T: Send>() {}
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send::<Scene>();
    assert_send::<Renderer>();
    assert_send_sync::<scena::NodeKey>();
    assert_send_sync::<scena::CameraKey>();
    assert_send_sync::<scena::SceneImport>();
}

#[test]
fn orbit_controls_are_platform_neutral_pointer_actions() {
    let mut controls = OrbitControls::new(Vec3::ZERO, 4.0);

    assert_eq!(
        controls.handle_pointer(PointerEvent {
            kind: PointerEventKind::Pressed,
            position: (10.0, 10.0),
            button: Some(PointerButton::Primary),
            delta: (0.0, 0.0),
            scroll_delta: 0.0,
        }),
        OrbitControlAction::BeginOrbit
    );
    assert_eq!(
        controls.handle_pointer(PointerEvent {
            kind: PointerEventKind::Moved,
            position: (30.0, 20.0),
            button: Some(PointerButton::Primary),
            delta: (20.0, 10.0),
            scroll_delta: 0.0,
        }),
        OrbitControlAction::Orbit
    );
    assert_eq!(
        controls.handle_pointer(PointerEvent {
            kind: PointerEventKind::Wheel,
            position: (30.0, 20.0),
            button: None,
            delta: (0.0, 0.0),
            scroll_delta: -1.0,
        }),
        OrbitControlAction::Zoom
    );
    assert!(controls.distance() < 4.0);
}
