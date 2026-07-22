use scena::{
    Assets, Backend, PerspectiveCamera, PrepareError, RenderError, Renderer, Scene, SurfaceEvent,
    Transform, Vec3,
};

fn prepared_headless_renderer() -> (Renderer, Scene, scena::CameraKey, Assets) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    let assets = Assets::new();
    let mut renderer = Renderer::headless(24, 24).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("initial render succeeds");
    (renderer, scene, camera, assets)
}

#[test]
fn injected_device_loss_requires_rebuild_and_rejects_prepare_and_render() {
    let (mut renderer, mut scene, camera, assets) = prepared_headless_renderer();
    renderer
        .handle_surface_event(SurfaceEvent::DeviceLost { recoverable: true })
        .expect("device-loss event records");

    assert_eq!(
        renderer.render(&scene, camera),
        Err(RenderError::GpuDeviceLost { recoverable: true })
    );
    let recovery = renderer
        .recover_context(&assets, &mut scene)
        .expect_err("a terminal wgpu device cannot be recovered as a context");
    assert!(matches!(
        recovery,
        PrepareError::GpuDeviceRebuildRequired {
            backend: Backend::Headless,
            recoverable: true,
        }
    ));
    assert!(recovery.help().contains("recreate the Renderer"));
    assert!(matches!(
        renderer.prepare_with_assets(&mut scene, &assets),
        Err(PrepareError::GpuDeviceRebuildRequired {
            backend: Backend::Headless,
            recoverable: true,
        })
    ));
    assert_eq!(
        renderer.render(&scene, camera),
        Err(RenderError::GpuDeviceLost { recoverable: true })
    );

    let mut replacement = Renderer::headless(24, 24).expect("replacement renderer builds");
    replacement
        .prepare_with_assets(&mut scene, &assets)
        .expect("retained scene and assets prepare on the replacement renderer");
    replacement
        .render(&scene, camera)
        .expect("replacement renderer submits a frame");
}

#[test]
fn repeated_device_loss_never_clears_the_terminal_state() {
    let (mut renderer, mut scene, camera, assets) = prepared_headless_renderer();
    for recoverable in [true, false, false] {
        renderer
            .handle_surface_event(SurfaceEvent::DeviceLost { recoverable })
            .expect("repeated device-loss event records");
        assert!(matches!(
            renderer.recover_context(&assets, &mut scene),
            Err(PrepareError::GpuDeviceRebuildRequired {
                backend: Backend::Headless,
                recoverable: observed,
            }) if observed == recoverable
        ));
        assert_eq!(
            renderer.render(&scene, camera),
            Err(RenderError::GpuDeviceLost { recoverable })
        );
    }
}

#[test]
fn device_loss_before_first_prepare_is_rejected_at_the_prepare_boundary() {
    let mut scene = Scene::new();
    let assets = Assets::new();
    let mut renderer = Renderer::headless(24, 24).expect("renderer builds");
    renderer
        .handle_surface_event(SurfaceEvent::DeviceLost { recoverable: true })
        .expect("device-loss event records");

    assert!(matches!(
        renderer.prepare_with_assets(&mut scene, &assets),
        Err(PrepareError::GpuDeviceRebuildRequired {
            backend: Backend::Headless,
            recoverable: true,
        })
    ));
}
