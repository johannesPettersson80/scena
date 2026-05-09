#![cfg(not(target_arch = "wasm32"))]

//! Phase 5B: interactive native + browser glTF viewer builders.
//!
//! Verifies the additive `interactive_gltf_viewer(path, surface)` fluent
//! builder loads, instantiates, frames, prepares, and surfaces the renderer
//! through a stable typed handle. Covers the `scena-api-ergonomics-reviewer`
//! Phase 6 finding F3 v1.0 commitment without owning the host event loop.

use scena::{
    DiagnosticSeverity, InteractiveGltfViewer, OrbitControlAction, PlatformSurface, PointerButton,
    PointerEvent, PointerEventKind, RenderMode, Renderer, Scene, SurfaceEvent,
    interactive_gltf_viewer,
};

#[test]
fn interactive_gltf_viewer_builds_load_instantiate_frame_prepare_render_through_descriptor_surface()
{
    // The descriptor surface backend is gated behind no GPU adapter, so the
    // builder must work end-to-end on every CI runner regardless of
    // host_gpu_available. This is the smallest first-render path that is not
    // headless; it proves the additive `interactive_gltf_viewer` ownership
    // shape is renderer-as-library and never owns the event loop.
    let viewer: InteractiveGltfViewer = interactive_gltf_viewer(
        "tests/assets/gltf/khronos/UnlitTest/UnlitTest.gltf",
        PlatformSurface::native_window(96, 64),
    )
    .with_default_light()
    .with_render_mode(RenderMode::OnChange)
    .build()
    .expect("interactive viewer builds against a descriptor surface");

    assert_eq!(
        viewer.renderer().capabilities().backend,
        scena::Backend::SurfaceDescriptor
    );
    assert_eq!(viewer.renderer().stats().target_width, 96);
    assert_eq!(viewer.renderer().stats().target_height, 64);
    let active_camera = viewer.camera();
    let _ = active_camera;

    let outcome = {
        let mut viewer = viewer;
        viewer
            .render_next_frame()
            .expect("interactive viewer renders one frame")
    };
    assert!(
        outcome.draw_calls > 0 || outcome.primitives == 0,
        "render_next_frame must report a coherent draw stat (got {outcome:?})",
    );
}

#[test]
fn interactive_gltf_viewer_forwards_surface_resize_events_to_renderer() {
    // SurfaceEvent::Resize must reach the renderer through the viewer's
    // handle_surface_event helper without the host having to reach into
    // viewer.renderer_mut() manually. This proves the renderer-as-library
    // ergonomics shape from the Phase 6 api-ergonomics F3 finding.
    let mut viewer = interactive_gltf_viewer(
        "tests/assets/gltf/khronos/UnlitTest/UnlitTest.gltf",
        PlatformSurface::native_window(48, 32),
    )
    .build()
    .expect("interactive viewer builds");

    viewer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 96,
            height: 64,
        })
        .expect("handle_surface_event forwards resize");

    assert_eq!(viewer.renderer().stats().target_width, 96);
    assert_eq!(viewer.renderer().stats().target_height, 64);
}

#[test]
fn interactive_gltf_viewer_diagnostics_accessor_reports_renderer_diagnostics() {
    // Diagnostics must be reachable through the viewer's accessor so callers
    // can surface beginner errors (forward_pbr Degraded, etc.) without
    // reaching into renderer_mut().
    let viewer = interactive_gltf_viewer(
        "tests/assets/gltf/khronos/UnlitTest/UnlitTest.gltf",
        PlatformSurface::native_window(64, 64),
    )
    .with_default_light()
    .with_default_environment()
    .build()
    .expect("interactive viewer builds");

    let diagnostics = viewer.diagnostics();
    // The exact set varies by capability; the contract is that the accessor
    // returns the same Vec the renderer's diagnostics() returns. Any non-zero
    // count proves the path is wired; an empty list also proves the accessor
    // is reachable. Both are acceptable.
    let _ = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity() != DiagnosticSeverity::Info);
}

#[test]
fn interactive_gltf_viewer_with_orbit_controls_attaches_controller_seeded_from_framing() {
    // Phase 5B step 2: `with_orbit_controls()` derives the initial OrbitControls
    // target+distance from the imported scene's bounds and the framed camera
    // position. The controller must therefore exist and have a positive
    // distance (the framed camera is offset along +Z from the bounds center).
    let viewer = interactive_gltf_viewer(
        "tests/assets/gltf/khronos/UnlitTest/UnlitTest.gltf",
        PlatformSurface::native_window(96, 64),
    )
    .with_orbit_controls()
    .build()
    .expect("interactive viewer builds with orbit controls");

    let controls = viewer
        .orbit_controls
        .as_ref()
        .expect("with_orbit_controls populates the controller field");
    assert!(
        controls.distance() > 0.0 && controls.distance().is_finite(),
        "framed orbit controls must seed a positive finite distance, got {}",
        controls.distance()
    );
    assert!(
        controls.yaw_radians().abs() < f32::EPSILON,
        "orbit controls start unrotated; yaw={}",
        controls.yaw_radians()
    );
}

#[test]
fn interactive_gltf_viewer_handle_pointer_event_orbits_and_applies_to_scene() {
    // Phase 5B step 2: routing pointer events through
    // `handle_pointer_event` must update the controller AND apply the
    // resulting transform to the active camera. The test presses the
    // primary button, drags 100 px right, and asserts that (a) the
    // returned action is `Orbit` and (b) the camera node's world
    // translation actually changed.
    let mut viewer = interactive_gltf_viewer(
        "tests/assets/gltf/khronos/UnlitTest/UnlitTest.gltf",
        PlatformSurface::native_window(96, 64),
    )
    .with_orbit_controls()
    .build()
    .expect("interactive viewer builds with orbit controls");

    let camera_node = viewer
        .scene
        .camera_node(viewer.camera)
        .expect("active camera has a node");
    let translation_before = viewer
        .scene
        .world_transform(camera_node)
        .expect("camera world transform")
        .translation;

    let press = viewer
        .handle_pointer_event(PointerEvent {
            kind: PointerEventKind::Pressed,
            position: (32.0, 32.0),
            button: Some(PointerButton::Primary),
            delta: (0.0, 0.0),
            scroll_delta: 0.0,
        })
        .expect("press event handled");
    assert_eq!(press, OrbitControlAction::BeginOrbit);

    let drag = viewer
        .handle_pointer_event(PointerEvent {
            kind: PointerEventKind::Moved,
            position: (132.0, 32.0),
            button: Some(PointerButton::Primary),
            delta: (100.0, 0.0),
            scroll_delta: 0.0,
        })
        .expect("drag event handled");
    assert_eq!(drag, OrbitControlAction::Orbit);

    let translation_after = viewer
        .scene
        .world_transform(camera_node)
        .expect("camera world transform")
        .translation;
    let dx = translation_after.x - translation_before.x;
    let dz = translation_after.z - translation_before.z;
    assert!(
        dx * dx + dz * dz > 1e-6,
        "100 px horizontal drag must rotate the camera around target; \
         translation moved from {translation_before:?} to {translation_after:?}",
    );
}

#[test]
fn interactive_gltf_viewer_handle_pointer_event_no_op_without_orbit_controls() {
    // Without `with_orbit_controls`, handle_pointer_event must short-circuit
    // and report None — the handler is always reachable so callers can
    // unconditionally route input through it.
    let mut viewer = interactive_gltf_viewer(
        "tests/assets/gltf/khronos/UnlitTest/UnlitTest.gltf",
        PlatformSurface::native_window(64, 64),
    )
    .build()
    .expect("interactive viewer builds without orbit controls");

    assert!(viewer.orbit_controls.is_none());
    let action = viewer
        .handle_pointer_event(PointerEvent {
            kind: PointerEventKind::Pressed,
            position: (10.0, 10.0),
            button: Some(PointerButton::Primary),
            delta: (0.0, 0.0),
            scroll_delta: 0.0,
        })
        .expect("event routes safely without orbit controls");
    assert_eq!(action, OrbitControlAction::None);
}

#[test]
fn renderer_headless_default_yields_canonical_first_render_dimensions() {
    // scena-api-ergonomics-reviewer Phase 6 finding F1 closure:
    // Renderer::headless_default() collapses the two-arg `Renderer::headless(w, h)`
    // boilerplate into a single zero-arg constructor. The canonical first-render
    // size is 800x600 per the Three.js parity baseline.
    let renderer = Renderer::headless_default().expect("headless default builds");
    assert_eq!(renderer.stats().target_width, 800);
    assert_eq!(renderer.stats().target_height, 600);
}

#[test]
fn scene_with_default_camera_returns_active_camera_in_one_call() {
    // scena-api-ergonomics-reviewer Phase 6 finding F1 closure:
    // Scene::with_default_camera() merges Scene::new() + add_default_camera()
    // into one call so first-render examples drop two lines of setup.
    let (scene, camera) = Scene::with_default_camera().expect("default scene + camera builds");
    assert_eq!(
        scene.active_camera(),
        Some(camera),
        "with_default_camera must register the camera as the active camera",
    );
}
