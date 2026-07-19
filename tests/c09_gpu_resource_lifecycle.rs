#![cfg(not(target_arch = "wasm32"))]

use scena::{
    AntiAliasing, Assets, CaptureError, DepthOfFieldConfig, DevicePollStatus, PerspectiveCamera,
    PostBloomConfig, PrepareError, Primitive, RenderError, RenderReadbackMode, Renderer,
    RendererStats, Scene, ScreenSpaceAmbientOcclusionConfig, ScreenSpaceReflectionConfig,
    SurfaceEvent, Transform, Vec3,
};

fn gpu_scene() -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("C09 triangle inserts");
    let camera = scene
        .add_default_camera()
        .expect("C09 default camera inserts");
    (scene, camera)
}

#[test]
fn msaa8_is_fully_prepared_or_rejected_before_render() {
    let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
        return;
    };
    let (mut scene, camera) = gpu_scene();
    disable_heavy_output(&mut renderer);
    renderer.prepare(&mut scene).expect("baseline prepares");
    let baseline = renderer.stats();

    renderer.set_anti_aliasing(AntiAliasing::Msaa8);
    match renderer.prepare(&mut scene) {
        Ok(()) => {
            let prepared = renderer.stats();
            assert_eq!(
                (
                    prepared.buffers,
                    prepared.gpu_textures,
                    prepared.render_targets,
                    prepared.pipelines,
                    prepared.bind_groups,
                    prepared.shader_modules,
                ),
                (10, 22, 6, 13, 6, 11),
                "supported MSAA8 owns its pipeline set, color target, and overlay depth set before render"
            );
            assert!(prepared.approximate_gpu_memory_bytes > baseline.approximate_gpu_memory_bytes);
            let signature = resource_signature(prepared);
            renderer
                .render(&scene, camera)
                .expect("prepared MSAA8 renders without lazy creation");
            assert_eq!(resource_signature(renderer.stats()), signature);
        }
        Err(PrepareError::UnsupportedSampleCount {
            requested, maximum, ..
        }) => {
            assert_eq!(requested, 8);
            assert!(maximum < requested);
        }
        Err(error) => panic!("unexpected MSAA8 prepare result: {error:?}"),
    }
}

fn resource_signature(stats: RendererStats) -> (u64, u64, u64, u64, u64, u64, Option<u64>) {
    (
        stats.buffers,
        stats.gpu_textures,
        stats.render_targets,
        stats.pipelines,
        stats.bind_groups,
        stats.shader_modules,
        stats.approximate_gpu_memory_bytes,
    )
}

fn enable_heavy_output(renderer: &mut Renderer) {
    renderer.set_anti_aliasing(AntiAliasing::Msaa4);
    renderer.set_bloom(Some(PostBloomConfig::subtle()));
    renderer.set_screen_space_ambient_occlusion(Some(ScreenSpaceAmbientOcclusionConfig::new(
        3, 0.5, 0.015,
    )));
    renderer.set_screen_space_reflections(Some(ScreenSpaceReflectionConfig::default()));
    renderer.set_depth_of_field(Some(DepthOfFieldConfig::new(2.5, 1.2, 6)));
}

fn disable_heavy_output(renderer: &mut Renderer) {
    renderer.set_anti_aliasing(AntiAliasing::None);
    renderer.clear_bloom();
    renderer.clear_screen_space_ambient_occlusion();
    renderer.clear_screen_space_reflections();
    renderer.clear_depth_of_field();
}

#[test]
fn output_resource_changes_require_prepare_and_stats_are_complete_before_render() {
    let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
        return;
    };
    let (mut scene, camera) = gpu_scene();
    disable_heavy_output(&mut renderer);
    renderer.prepare(&mut scene).expect("baseline prepares");
    let baseline = renderer.stats();
    assert_eq!(
        (
            baseline.buffers,
            baseline.gpu_textures,
            baseline.render_targets,
            baseline.pipelines,
            baseline.bind_groups,
            baseline.shader_modules,
        ),
        (10, 20, 4, 9, 6, 8),
        "the simple native baseline inventories core, light assignment, fallback material, shadow/environment, transmission, and depth owners exactly"
    );
    assert!(
        baseline.gpu_textures > 0,
        "baseline GPU texture owners are counted"
    );

    enable_heavy_output(&mut renderer);
    assert!(
        matches!(
            renderer.render(&scene, camera),
            Err(RenderError::NotPrepared { .. })
        ),
        "resource-affecting AA/post changes must invalidate prepared output resources"
    );

    renderer
        .prepare(&mut scene)
        .expect("MSAA/post resources prepare explicitly");
    let prepared = renderer.stats();
    assert_eq!(
        (
            prepared.buffers,
            prepared.gpu_textures,
            prepared.render_targets,
            prepared.pipelines,
            prepared.bind_groups,
            prepared.shader_modules,
        ),
        (12, 27, 11, 21, 12, 19),
        "MSAA4 + post + command-ordered uniform staging + depth-color + overlay owners are additive and exact"
    );
    assert!(
        prepared.buffers == baseline.buffers + 2,
        "post owns both its live uniform and a command-ordered staging buffer"
    );
    assert!(
        prepared.render_targets > baseline.render_targets,
        "MSAA, post, and depth-color targets must be counted before render"
    );
    assert!(
        prepared.gpu_textures > baseline.gpu_textures,
        "post, MSAA, and depth-color texture owners must be counted before render"
    );
    assert!(
        prepared.pipelines > baseline.pipelines,
        "post and MSAA pipelines must be counted before render"
    );
    assert!(
        prepared.bind_groups > baseline.bind_groups,
        "post bind groups must be counted before render"
    );
    assert!(
        prepared.shader_modules > baseline.shader_modules,
        "post and MSAA shader modules must be counted before render"
    );
    assert!(
        prepared.approximate_gpu_memory_bytes > baseline.approximate_gpu_memory_bytes,
        "post, MSAA, and depth-color texture bytes must be included before render"
    );

    let before_render = resource_signature(prepared);
    renderer
        .render(&scene, camera)
        .expect("fully prepared heavy output renders");
    assert_eq!(
        resource_signature(renderer.stats()),
        before_render,
        "render must not create, destroy, or revise reported GPU resources"
    );

    disable_heavy_output(&mut renderer);
    renderer
        .prepare(&mut scene)
        .expect("disabling heavy output reprepares baseline resources");
    let released = renderer.stats();
    assert_eq!(
        resource_signature(released),
        resource_signature(baseline),
        "disabling all optional output resources must return exact counters to baseline"
    );
    assert_eq!(
        released.pending_destructions,
        prepared.buffers + prepared.gpu_textures + prepared.pipelines + prepared.bind_groups,
        "pending destruction counts each tracked live GPU object exactly once; render targets are textures and prepared shader modules are compilation inputs, not retained objects"
    );
    let poll = renderer.poll_device();
    assert_eq!(poll.status, DevicePollStatus::Confirmed);
    assert_eq!(
        poll.pending_destructions_before,
        released.pending_destructions
    );
    assert_eq!(poll.destroyed_resources, released.pending_destructions);
    assert_eq!(poll.pending_destructions_after, 0);
}

#[test]
fn cpu_poll_reports_explicitly_unsupported_instead_of_success() {
    let mut renderer = Renderer::headless(4, 4).expect("CPU renderer builds");
    let poll = renderer.poll_device();
    assert_eq!(poll.status, DevicePollStatus::Unsupported);
    assert_eq!(poll.destroyed_resources, 0);
    assert!(!poll.gpu_polled);
}

#[test]
fn output_revision_and_native_readback_modes_are_explicit_and_render_allocates_no_gpu_resources() {
    let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
        return;
    };
    let (mut scene, camera) = gpu_scene();
    disable_heavy_output(&mut renderer);
    renderer.prepare(&mut scene).expect("baseline prepares");

    enable_heavy_output(&mut renderer);
    assert!(matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared {
            reason: scena::NotPreparedReason::OutputSettingsChanged { .. }
        })
    ));
    renderer
        .prepare(&mut scene)
        .expect("output resources prepare before either render mode");

    renderer
        .render_with_readback_mode(&scene, camera, RenderReadbackMode::PresentOnly)
        .expect("present-only render succeeds");
    let present = renderer.last_render_work_metrics();
    assert_eq!(present.readback_copies, 0);
    assert_eq!(present.map_requests, 0);
    assert_eq!(present.blocking_polls, 0);
    assert_eq!(present.blocking_waits, 0);
    assert_eq!(present.cpu_frame_copy_bytes, 0);
    assert_eq!(present.gpu_buffer_creations, 0);
    assert_eq!(present.gpu_texture_creations, 0);
    assert_eq!(present.gpu_pipeline_creations, 0);
    assert_eq!(present.gpu_bind_group_creations, 0);
    assert_eq!(present.gpu_shader_module_creations, 0);
    assert!(matches!(
        renderer.capture_rgba8(&scene, Default::default()),
        Err(CaptureError::NoRenderedFrame)
    ));

    renderer
        .render_with_readback_mode(&scene, camera, RenderReadbackMode::Synchronous)
        .expect("synchronous capture render succeeds");
    let synchronous = renderer.last_render_work_metrics();
    assert_eq!(synchronous.readback_copies, 1);
    assert_eq!(synchronous.map_requests, 1);
    assert_eq!(synchronous.blocking_polls, 1);
    assert_eq!(synchronous.blocking_waits, 1);
    assert_eq!(synchronous.cpu_frame_copy_bytes, 16 * 16 * 4);
    assert_eq!(synchronous.gpu_buffer_creations, 0);
    assert_eq!(synchronous.gpu_texture_creations, 0);
    assert_eq!(synchronous.gpu_pipeline_creations, 0);
    assert_eq!(synchronous.gpu_bind_group_creations, 0);
    assert_eq!(synchronous.gpu_shader_module_creations, 0);
    assert!(renderer.capture_rgba8(&scene, Default::default()).is_ok());
}

#[test]
fn double_buffered_async_readback_batch_preserves_input_order() {
    let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
        return;
    };
    let (mut scene, camera) = gpu_scene();
    let alternate = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(8.0, 0.0, 3.0)),
        )
        .expect("alternate C09 camera inserts");
    disable_heavy_output(&mut renderer);
    renderer.prepare(&mut scene).expect("batch scene prepares");

    let frames = renderer
        .render_batch_with_async_readback(&scene, &[camera, alternate, camera])
        .expect("double-buffered batch capture succeeds");
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].rgba8(), frames[2].rgba8());
    assert_ne!(frames[0].rgba8(), frames[1].rgba8());
    assert_eq!(renderer.frame_rgba8(), frames[2].rgba8());

    let metrics = renderer.last_render_work_metrics();
    assert_eq!(metrics.readback_copies, 3);
    assert_eq!(metrics.map_requests, 3);
    assert_eq!(metrics.async_readback_submissions, 3);
    assert_eq!(metrics.peak_readbacks_in_flight, 2);
    assert_eq!(metrics.gpu_buffer_creations, 0);
    assert_eq!(metrics.gpu_texture_creations, 0);
    assert_eq!(metrics.gpu_pipeline_creations, 0);
    assert_eq!(metrics.gpu_bind_group_creations, 0);
    assert_eq!(metrics.gpu_shader_module_creations, 0);
}

#[test]
fn resize_and_context_recovery_rebuild_the_same_resource_shape() {
    let Ok(mut renderer) = Renderer::headless_gpu(16, 16) else {
        return;
    };
    let assets = Assets::new();
    let (mut scene, camera) = gpu_scene();
    enable_heavy_output(&mut renderer);
    renderer.prepare(&mut scene).expect("heavy output prepares");
    let initial = renderer.stats();

    renderer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 32,
            height: 16,
        })
        .expect("target resizes");
    renderer
        .prepare(&mut scene)
        .expect("resized output prepares");
    let resized = renderer.stats();
    assert_eq!(
        (
            resized.buffers,
            resized.gpu_textures,
            resized.render_targets,
            resized.pipelines,
            resized.bind_groups,
            resized.shader_modules,
        ),
        (
            initial.buffers,
            initial.gpu_textures,
            initial.render_targets,
            initial.pipelines,
            initial.bind_groups,
            initial.shader_modules,
        ),
        "resize must rebuild the same owner shape"
    );
    assert!(
        resized.approximate_gpu_memory_bytes > initial.approximate_gpu_memory_bytes,
        "doubling target pixels must increase exact texture/buffer byte accounting"
    );
    let before_render = resource_signature(resized);
    renderer
        .render(&scene, camera)
        .expect("resized prepared output renders");
    assert_eq!(resource_signature(renderer.stats()), before_render);

    renderer
        .handle_surface_event(SurfaceEvent::ContextLost { recoverable: true })
        .expect("context loss records");
    renderer
        .handle_surface_event(SurfaceEvent::ContextRestored)
        .expect("context restore records");
    renderer
        .recover_context(&assets, &mut scene)
        .expect("retained state recovers");
    renderer
        .prepare(&mut scene)
        .expect("recovered output resources prepare");
    assert_eq!(
        resource_signature(renderer.stats()),
        resource_signature(resized),
        "context recovery must rebuild the complete optional resource shape"
    );
}
