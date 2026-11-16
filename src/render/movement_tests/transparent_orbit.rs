//! Orbiting the camera around alpha-blended and transmissive geometry.
//!
//! Blended triangles are drawn farthest first, and "farthest" depends on the
//! camera. That used to send every camera move in a scene with one blended
//! mesh through a full prepare. The retained path now re-sorts the blended
//! triangles for the new camera instead, and these tests pin both halves of
//! that: no full prepare after the first frame, and every frame equal to the
//! one a full prepare from that camera produces.

use crate::assets::Assets;
use crate::geometry::GeometryDesc;
use crate::material::{AlphaMode, Color, MaterialDesc};
use crate::scene::{CameraKey, DirectionalLight, NodeKey, Scene, Transform, Vec3};

use super::support::{
    Backend, PrepareWork, frame_difference, fresh_full_prepare_frame, orbit_camera, pixel,
    prepare_and_render, retained_frame_after,
};

const SIZE: u32 = 96;
const ORBIT_RADIUS: f32 = 2.2;
/// A pixel covered by both blended panes but not by the opaque box between
/// them, from the front (yaw 0) and from the back (yaw 180).
const BOTH_PANES: (u32, u32) = (58, 48);

struct PaneScene {
    scene: Scene,
    assets: Assets,
    camera: CameraKey,
    red_pane: NodeKey,
    box_node: NodeKey,
}

/// A red pane in front of the origin and a blue pane behind it, both half
/// transparent and double sided, with a small opaque box between them.
///
/// `lit` picks physically lit materials (shaded on the GPU, baked per vertex
/// with the view vector by the CPU renderer) or unlit ones (no view-dependent
/// shading on either backend).
fn pane_scene(lit: bool) -> PaneScene {
    let assets = Assets::new();
    let material = |color: Color, alpha_mode: AlphaMode| {
        let desc = if lit {
            MaterialDesc::pbr_metallic_roughness(color, 0.0, 0.7)
        } else {
            MaterialDesc::unlit(color)
        };
        assets.create_material(desc.with_alpha_mode(alpha_mode).with_double_sided(true))
    };
    let pane = assets.create_geometry(GeometryDesc::plane(1.0, 1.0));
    let cube = assets.create_geometry(GeometryDesc::box_xyz(0.2, 0.2, 0.2));
    let red = material(
        Color::from_linear_rgba(1.0, 0.05, 0.05, 0.5),
        AlphaMode::Blend,
    );
    let blue = material(
        Color::from_linear_rgba(0.05, 0.05, 1.0, 0.5),
        AlphaMode::Blend,
    );
    let grey = material(Color::from_linear_rgb(0.6, 0.6, 0.6), AlphaMode::Opaque);

    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    orbit_camera(&mut scene, camera, 0.0, ORBIT_RADIUS);
    // One light from each side, so both faces of every pane are lit.
    for yaw in [0.0, 180.0] {
        scene
            .directional_light(DirectionalLight::default())
            .transform(Transform::IDENTITY.rotate_y_deg(yaw))
            .add()
            .expect("light inserts");
    }
    let facing_z = |z: f32| Transform::at(Vec3::new(0.0, 0.0, z)).rotate_x_deg(90.0);
    let red_pane = scene
        .mesh(pane, red)
        .transform(facing_z(0.5))
        .add()
        .expect("red pane inserts");
    scene
        .mesh(pane, blue)
        .transform(facing_z(-0.5))
        .add()
        .expect("blue pane inserts");
    let box_node = scene.mesh(cube, grey).add().expect("opaque box inserts");
    PaneScene {
        scene,
        assets,
        camera,
        red_pane,
        box_node,
    }
}

/// Orbits `backend` around the panes and checks every frame after the first.
fn assert_orbit_reuses_the_retained_template(backend: Backend, lit: bool) {
    let mut fixture = pane_scene(lit);
    let Some(mut renderer) = backend.renderer(SIZE, SIZE) else {
        return;
    };
    let (first, first_work) = prepare_and_render(
        &mut renderer,
        &mut fixture.scene,
        &fixture.assets,
        fixture.camera,
    );
    assert_eq!(
        first_work.full_prepares, 1,
        "the first frame bakes the scene"
    );
    let front = pixel(&first, SIZE, BOTH_PANES.0, BOTH_PANES.1);
    assert!(
        front[0] > front[2] + 15,
        "{backend:?}: from the front the red pane is nearest and must blend last: {front:?}"
    );

    for yaw in [35.0, 140.0, 180.0, 255.0, 330.0] {
        orbit_camera(&mut fixture.scene, fixture.camera, yaw, ORBIT_RADIUS);
        let (frame, work) = prepare_and_render(
            &mut renderer,
            &mut fixture.scene,
            &fixture.assets,
            fixture.camera,
        );
        assert_eq!(
            work,
            PrepareWork::RETAINED,
            "{backend:?} yaw {yaw}: a camera move must not bake the scene again"
        );
        let reference = fresh_full_prepare_frame(
            backend,
            SIZE,
            SIZE,
            &mut fixture.scene,
            &fixture.assets,
            fixture.camera,
        );
        assert_eq!(
            frame_difference(&frame, &reference),
            (0, 0),
            "{backend:?} yaw {yaw}: (differing pixels, largest channel difference) against a \
             full prepare from the same camera must be zero"
        );
        if yaw == 180.0 {
            let back = pixel(&frame, SIZE, BOTH_PANES.0, BOTH_PANES.1);
            assert!(
                back[2] > back[0] + 15,
                "{backend:?}: from behind the blue pane is nearest and must blend last, which \
                 only holds if the panes were re-sorted for this camera: {back:?}"
            );
        }
    }
}

#[test]
fn orbiting_blended_panes_re_sorts_them_without_a_full_prepare_on_the_gpu() {
    assert_orbit_reuses_the_retained_template(Backend::Gpu, true);
}

#[test]
fn orbiting_unlit_blended_panes_re_sorts_them_without_a_full_prepare_on_the_cpu() {
    assert_orbit_reuses_the_retained_template(Backend::Cpu, false);
}

#[test]
fn orbiting_lit_panes_on_the_cpu_still_bakes_view_dependent_shading_again() {
    // The CPU renderer shades lit materials per vertex with the view vector
    // at prepare time. Reusing that bake from another camera would freeze the
    // highlights, so this case must keep its full prepare.
    let mut fixture = pane_scene(true);
    let camera = fixture.camera;
    let Some(retained) = retained_frame_after(
        Backend::Cpu,
        SIZE,
        &mut fixture.scene,
        &fixture.assets,
        |scene| orbit_camera(scene, camera, 140.0, ORBIT_RADIUS),
    ) else {
        return;
    };

    assert_eq!(
        (retained.work.full_prepares, retained.work.retained_prepares),
        (1, 0),
        "view-dependent CPU shading cannot be reused from another camera"
    );
    retained.assert_matches_full_prepare("lit panes on the CPU renderer");
}

#[test]
fn orbiting_a_transmissive_box_reuses_the_retained_template_on_the_gpu() {
    let assets = Assets::new();
    let glass = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_linear_rgb(0.8, 0.95, 1.0), 0.0, 0.1)
            .with_transmission_factor(0.9)
            .with_ior(1.45)
            .with_thickness_factor(0.2),
    );
    let solid = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_linear_rgb(0.9, 0.3, 0.1),
        0.0,
        0.6,
    ));
    let shell = assets.create_geometry(GeometryDesc::box_xyz(0.7, 0.7, 0.7));
    let core = assets.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    orbit_camera(&mut scene, camera, 20.0, ORBIT_RADIUS);
    scene
        .directional_light(DirectionalLight::default())
        .transform(Transform::IDENTITY.rotate_x_deg(-30.0).rotate_y_deg(25.0))
        .add()
        .expect("light inserts");
    scene.mesh(shell, glass).add().expect("glass shell inserts");
    scene
        .mesh(core, solid)
        .transform(Transform::at(Vec3::new(0.1, 0.0, -0.6)))
        .add()
        .expect("opaque core inserts");

    let Some(mut renderer) = Backend::Gpu.renderer(SIZE, SIZE) else {
        return;
    };
    prepare_and_render(&mut renderer, &mut scene, &assets, camera);
    for yaw in [110.0, 200.0, 290.0] {
        orbit_camera(&mut scene, camera, yaw, ORBIT_RADIUS);
        let (frame, work) = prepare_and_render(&mut renderer, &mut scene, &assets, camera);
        assert_eq!(
            work,
            PrepareWork::RETAINED,
            "yaw {yaw}: a camera move around transmissive geometry must not bake the scene again"
        );
        let reference =
            fresh_full_prepare_frame(Backend::Gpu, SIZE, SIZE, &mut scene, &assets, camera);
        assert_eq!(
            frame_difference(&frame, &reference),
            (0, 0),
            "yaw {yaw}: the retained transmissive frame must equal a full prepare"
        );
    }
}

#[test]
fn moving_blended_and_opaque_meshes_keeps_the_blend_order_of_a_full_prepare_on_the_gpu() {
    // The opaque box moves while the panes stay, then a pane itself moves to
    // the far side of the other one, which swaps their blend order.
    for red_pane_moves in [false, true] {
        let PaneScene {
            mut scene,
            assets,
            red_pane,
            box_node,
            ..
        } = pane_scene(true);
        let case = if red_pane_moves {
            "red pane moved behind the blue pane"
        } else {
            "opaque box moved behind both panes"
        };
        let Some(retained) =
            retained_frame_after(Backend::Gpu, SIZE, &mut scene, &assets, |scene| {
                let (node, transform) = if red_pane_moves {
                    (
                        red_pane,
                        Transform::at(Vec3::new(0.1, 0.0, -0.9)).rotate_x_deg(90.0),
                    )
                } else {
                    (box_node, Transform::at(Vec3::new(0.3, 0.2, -0.9)))
                };
                scene.set_transform(node, transform).expect("node moves");
            })
        else {
            return;
        };
        assert_eq!(retained.work, PrepareWork::RETAINED, "{case}");
        retained.assert_matches_full_prepare_within_rounding(case);
        if red_pane_moves {
            let both = pixel(&retained.frame, SIZE, BOTH_PANES.0, BOTH_PANES.1);
            assert!(
                both[2] > both[0] + 15,
                "{case}: the blue pane is now nearest and must blend last, which only holds \
                 if the panes were sorted where they are now: {both:?}"
            );
        }
    }
}
