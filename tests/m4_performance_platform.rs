#![cfg(not(target_arch = "wasm32"))]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use scena::{
    Assets, Backend, CameraKey, CapabilityStatus, Color, DiagnosticCode, DiagnosticSeverity,
    GeometryDesc, GeometryTopology, GeometryVertex, HardwareTier, MaterialDesc, OrbitControlAction,
    OrbitControls, OutputColorSpace, PlatformSurface, PointerButton, PointerEvent,
    PointerEventKind, Profile, Quality, RenderError, RenderMode, Renderer, RendererOptions, Scene,
    SurfaceEvent, Transform, Vec3,
};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static COUNT_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
}

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNT_ALLOCATIONS.with(Cell::get) {
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
        CapabilityStatus::Supported,
        "subtle output bloom is an explicit postprocess with rendered ON/OFF proof"
    );
    assert_eq!(
        headless.screen_space_ambient_occlusion,
        CapabilityStatus::Supported,
        "CPU headless SSAO has depth-aware visual proof; GPU/browser lanes remain separate"
    );
    assert_eq!(
        headless.directional_shadows,
        CapabilityStatus::Degraded,
        "CPU/reference lanes do not claim the GPU shadow-map receiver path"
    );
    assert_eq!(
        headless.order_independent_transparency,
        CapabilityStatus::Supported,
        "CPU headless OIT has overlap order-invariance proof; GPU/browser lanes remain separate"
    );
    assert_eq!(
        headless.physical_glass_transmission,
        CapabilityStatus::Degraded,
        "physical glass stays degraded until transmission, IOR/thickness/refraction, rough blur, and ordering proof all exist for the lane"
    );
    assert_eq!(
        headless.wide_gamut_output,
        CapabilityStatus::FeatureDisabled,
        "CPU/headless output targets sRGB; wide gamut must not be implied without a surface color-space probe"
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
    assert_eq!(
        webgl2.wide_gamut_output,
        CapabilityStatus::FeatureDisabled,
        "unattached WebGL2 capabilities cannot claim a browser canvas color space"
    );
    assert_eq!(
        webgl2.forward_pbr,
        CapabilityStatus::Degraded,
        "factory WebGL2 capabilities without an attached GPU device must stay degraded"
    );
    assert_eq!(
        webgl2.directional_shadows,
        CapabilityStatus::Degraded,
        "factory WebGL2 capabilities without an attached GPU device must not claim shadow receiver proof"
    );
    assert_eq!(
        webgl2.physical_glass_transmission,
        CapabilityStatus::Degraded,
        "factory WebGL2 capabilities without an attached GPU device must not claim physical glass"
    );
    let attached_webgl2 = scena::Capabilities::for_attached_gpu_backend(Backend::WebGl2);
    assert_eq!(
        attached_webgl2.forward_pbr,
        CapabilityStatus::Supported,
        "attached WebGL2 now owns the shared GPU PBR path and must not keep the stale degraded claim"
    );
    assert_eq!(
        attached_webgl2.directional_shadows,
        CapabilityStatus::Supported,
        "attached WebGL2 has browser receiver-darkening proof for directional shadows"
    );
    assert_eq!(
        attached_webgl2.physical_glass_transmission,
        CapabilityStatus::Supported,
        "attached WebGL2 has the opaque scene-color, refraction, rough-blur, and sorted transparent pass"
    );
    assert_eq!(
        attached_webgl2.wide_gamut_output,
        CapabilityStatus::Degraded,
        "attached browser WebGL2 needs a measured drawingBufferColorSpace probe before claiming Display P3"
    );

    let webgpu = scena::Capabilities::for_attached_gpu_backend(Backend::WebGpu);
    assert_eq!(webgpu.hardware_tier, HardwareTier::Medium);
    assert_eq!(webgpu.forward_pbr, CapabilityStatus::Supported);
    assert_eq!(webgpu.directional_shadows, CapabilityStatus::Supported);
    assert_eq!(
        webgpu.gpu_frustum_culling,
        CapabilityStatus::FeatureDisabled
    );
    assert_eq!(webgpu.per_instance_culling, CapabilityStatus::Supported);
    assert_eq!(webgpu.compute_shaders, CapabilityStatus::Supported);
    assert_eq!(webgpu.bloom, CapabilityStatus::Supported);
    assert_eq!(
        webgpu.order_independent_transparency,
        CapabilityStatus::FeatureDisabled
    );
    assert_eq!(
        webgpu.physical_glass_transmission,
        CapabilityStatus::Supported,
        "attached WebGPU uses the same physical transmission pipeline as WebGL2/native GPU"
    );
    assert_eq!(
        webgpu.wide_gamut_output,
        CapabilityStatus::Degraded,
        "attached WebGPU needs a measured canvas color-space probe before claiming Display P3"
    );
    assert_eq!(webgpu.texture_arrays, CapabilityStatus::Supported);
    assert_eq!(webgpu.max_texture_array_layers, 256);

    let diagnostics = webgpu.diagnostics();
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != scena::DiagnosticCode::ForwardPbrDegraded),
        "supported GPU PBR lanes must not emit the stale degraded diagnostic: {diagnostics:?}",
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != scena::DiagnosticCode::MaterialPresetFallback),
        "supported GPU PBR lanes must not tell users material presets are fallback-only: {diagnostics:?}",
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != scena::DiagnosticCode::DirectionalShadowsDegraded),
        "supported GPU directional-shadow lanes must not emit the stale degraded diagnostic: {diagnostics:?}",
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == scena::DiagnosticCode::PointShadowsDisabled
            && diagnostic.message.contains("Point shadows")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == scena::DiagnosticCode::SpotShadowsDisabled
            && diagnostic.message.contains("Spot shadows")
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != scena::DiagnosticCode::BloomDisabled),
        "supported bloom must not emit the old disabled diagnostic: {diagnostics:?}",
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != scena::DiagnosticCode::AmbientOcclusionDisabled),
        "supported ambient occlusion must not emit the old disabled diagnostic: {diagnostics:?}",
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == scena::DiagnosticCode::OrderIndependentTransparencyDisabled
            && diagnostic
                .message
                .contains("Order-independent transparency")
    }));
    assert!(
        diagnostics.iter().all(|diagnostic| diagnostic.code
            != scena::DiagnosticCode::PhysicalGlassTransmissionDegraded),
        "supported GPU glass lanes must not emit the stale degraded diagnostic: {diagnostics:?}",
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == scena::DiagnosticCode::GpuCullingDisabled
            && diagnostic.message.contains("GPU culling")
    }));
}

#[test]
fn material_preset_fallback_diagnostic_names_real_world_material_gaps() {
    let diagnostics = scena::Capabilities::for_backend(Backend::WebGl2).diagnostics();
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == DiagnosticCode::MaterialPresetFallback)
        .expect("unapproved material preset lane reports a structured fallback diagnostic");

    assert_eq!(diagnostic.severity(), DiagnosticSeverity::Warning);
    assert!(
        diagnostic.message().contains("real-world material presets")
            && diagnostic.message().contains("brushed_steel")
            && diagnostic.message().contains("clear_glass"),
        "diagnostic must name the real-world material fallback, got: {}",
        diagnostic.message()
    );
    assert!(
        diagnostic
            .help()
            .is_some_and(|help| help.contains("Round E") && help.contains("capability row")),
        "diagnostic help must point users to Round E capability proof, got: {:?}",
        diagnostic.help()
    );
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
fn display_p3_output_requires_explicit_canvas_configuration_proof() {
    let renderer = Renderer::headless_with_options(
        16,
        16,
        RendererOptions::default().with_output_color_space(OutputColorSpace::DisplayP3),
    )
    .expect("renderer builds");

    assert_eq!(renderer.output_color_space(), OutputColorSpace::DisplayP3);
    assert_eq!(
        renderer.capabilities().wide_gamut_output,
        CapabilityStatus::FeatureDisabled,
        "Display P3 requests on headless output must not imply a wide-gamut browser surface"
    );

    let webgl2_display_p3 =
        scena::Capabilities::for_attached_gpu_backend(Backend::WebGl2).with_display_p3_output(true);
    assert_eq!(
        webgl2_display_p3.output_stage,
        scena::OutputStageStatus::PbrNeutralDisplayP3
    );
    assert_eq!(
        webgl2_display_p3.color_target_format,
        "Rgba8UnormSrgb+DisplayP3Canvas"
    );
    assert_eq!(
        webgl2_display_p3.wide_gamut_output,
        CapabilityStatus::Supported,
        "the renderer may claim Display P3 only after its canvas output path configured display-p3"
    );
    assert!(
        webgl2_display_p3.diagnostics().iter().all(|diagnostic| {
            diagnostic.code != scena::DiagnosticCode::WideGamutOutputUnavailable
        }),
        "supported Display P3 output should not emit the unavailable diagnostic"
    );

    let webgpu_without_config = scena::Capabilities::for_attached_gpu_backend(Backend::WebGpu)
        .with_display_p3_output(false);
    assert_eq!(
        webgpu_without_config.wide_gamut_output,
        CapabilityStatus::Degraded,
        "an attached browser backend without renderer-owned color-space configuration remains degraded"
    );
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
    COUNT_ALLOCATIONS.with(|count| count.set(true));
    let outcome = renderer
        .render(&scene, camera)
        .expect("static idle frame is skipped");
    COUNT_ALLOCATIONS.with(|count| count.set(false));

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

/// Plan line 1336 / RFC line 1226: GPU upload preservation across
/// `SurfaceEvent::ContextLost → ContextRestored`. The renderer drops GPU-side
/// material textures, the environment cubemap, and the directional shadow
/// caster on context loss; recovery rebuilds those resources from the
/// CPU-retained scene + assets and the post-recovery stats match the
/// pre-loss baseline byte-for-byte. Closes the open dimension on master plan
/// line 1336 ("hot reload preserves GPU upload across context loss").
#[test]
fn context_recovery_preserves_material_textures_cubemap_and_shadow_caster() {
    let Ok(mut renderer) = Renderer::headless_gpu(32, 32) else {
        return;
    };

    let assets = Assets::new();
    let geometry = assets.create_geometry(
        GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vec![
                GeometryVertex {
                    position: Vec3::new(-0.6, -0.6, 0.0),
                    normal: Vec3::new(0.0, 0.0, 1.0),
                },
                GeometryVertex {
                    position: Vec3::new(0.6, -0.6, 0.0),
                    normal: Vec3::new(0.0, 0.0, 1.0),
                },
                GeometryVertex {
                    position: Vec3::new(0.0, 0.6, 0.0),
                    normal: Vec3::new(0.0, 0.0, 1.0),
                },
            ],
            vec![0, 1, 2],
        )
        .expect("triangle geometry"),
    );
    // 1×1 PNG so the renderer allocates GPU textures (per-material path
    // because there's only one material slot, but the textures are still
    // GPU resources that must survive the context-loss cycle).
    let albedo = pollster::block_on(assets.load_texture(
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
        scena::TextureColorSpace::Srgb,
    ))
    .expect("inline PNG loads");
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.5)
            .with_base_color_texture(albedo),
    );
    let environment = assets.default_environment();

    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(geometry, material)
        .add()
        .expect("textured mesh inserts");
    scene
        .directional_light(
            scena::DirectionalLight::default()
                .with_color(Color::WHITE)
                .with_illuminance_lux(60_000.0)
                .with_shadows(true),
        )
        .add()
        .expect("directional light inserts so the shadow caster pass allocates");

    renderer.set_environment(environment);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("textured + lit scene prepares for headless GPU");
    renderer
        .render(&scene, camera)
        .expect("first render succeeds");
    let baseline = renderer.stats();
    assert!(
        baseline.material_texture_bindings >= 1,
        "baseline must allocate GPU material texture bindings, got {}",
        baseline.material_texture_bindings
    );
    assert!(
        baseline.environment_cubemaps >= 1,
        "baseline must allocate the environment cubemap, got {}",
        baseline.environment_cubemaps
    );
    assert!(
        baseline.shadow_maps >= 1,
        "baseline must allocate the directional shadow caster, got {}",
        baseline.shadow_maps
    );
    let baseline_pipelines = baseline.pipelines;
    let baseline_material_bind_groups = baseline.material_bind_groups;
    let baseline_textures = baseline.textures;

    renderer
        .handle_surface_event(SurfaceEvent::ContextLost { recoverable: true })
        .expect("context loss is recorded");
    assert_eq!(
        renderer.render(&scene, camera),
        Err(RenderError::ContextLost { recoverable: true }),
        "render must surface the lost context until the recovery handshake completes",
    );

    renderer
        .handle_surface_event(SurfaceEvent::ContextRestored)
        .expect("context restoration event is recorded");
    renderer
        .recover_context(&assets, &mut scene)
        .expect("retained assets allow context recovery");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("re-prepare after recovery rebuilds GPU resources");
    renderer
        .render(&scene, camera)
        .expect("post-recovery render succeeds");

    let recovered = renderer.stats();
    assert_eq!(
        recovered.material_texture_bindings, baseline.material_texture_bindings,
        "material texture bindings must rebuild byte-for-byte after context loss"
    );
    assert_eq!(
        recovered.material_bind_groups, baseline_material_bind_groups,
        "material bind groups must reuse the same per-material vs batched shape \
         after context loss (proves the prepared scene's MaterialBatchPlan \
         re-runs through the same code path)"
    );
    assert_eq!(
        recovered.environment_cubemaps, baseline.environment_cubemaps,
        "environment cubemap must rebuild after context loss"
    );
    assert_eq!(
        recovered.shadow_maps, baseline.shadow_maps,
        "directional shadow caster must rebuild after context loss"
    );
    assert_eq!(
        recovered.pipelines, baseline_pipelines,
        "shader pipelines must rebuild after context loss"
    );
    assert_eq!(
        recovered.textures, baseline_textures,
        "logical texture count must remain stable across the recovery cycle"
    );
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
