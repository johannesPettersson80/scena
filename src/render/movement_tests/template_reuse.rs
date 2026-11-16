//! Which changes the retained template absorbs, and which bake the scene again.
//!
//! Every case prepares a scene, changes one thing, prepares again on the same
//! renderer and compares the frame with a renderer that has never seen the
//! scene. A case that expects the template to be reused also pins that no full
//! prepare ran. A case that expects a bake pins the reason for it, so a change
//! that starts baking for some other reason does not pass by accident.
//!
//! The cases here change the view, a tint or what is visible. Cases about
//! content that is baked for one view or one placement are in `baked_content`.

use crate::assets::{TextureMemoryDesc, TextureMemoryId, TextureSlot};
use crate::geometry::Aabb;
use crate::material::{Color, MaterialDesc};
use crate::scene::{PerspectiveCamera, Transform, Vec3};

use super::super::prepare_lifecycle::FullPrepareReason as Reason;
use super::reuse_stage::{BACKENDS, SIZE, Stage, move_to, orbit_to, set_visible, tint};
use super::support::{
    Backend, PrepareWork, frame_difference, fresh_full_prepare_frame, prepare_and_render,
};

#[test]
fn an_unchanged_scene_a_tint_and_a_hidden_node_reuse_the_template() {
    for backend in BACKENDS {
        let mut stage = Stage::for_backend(backend);
        let (subject, neighbour) = (stage.subject, stage.neighbour);
        let Some(unchanged) = stage.after(backend, |_scene| {}) else {
            return;
        };
        unchanged.assert_retained(&format!("{backend:?}: nothing changed"));
        stage
            .then(backend, tint(subject))
            .assert_retained(&format!("{backend:?}: tinted box"));
        stage
            .then(backend, set_visible(neighbour, false))
            .assert_retained(&format!("{backend:?}: hidden neighbour"));
        // The renderer of this case first saw the scene without the
        // neighbour, so its template has nothing to draw it from.
        stage
            .then(backend, set_visible(neighbour, true))
            .assert_full_prepare(
                Reason::TemplateCoverage,
                &format!("{backend:?}: neighbour shown for the first time"),
            );
    }
}

#[test]
fn the_cpu_renderer_keeps_lit_bakes_only_while_their_inputs_stand() {
    let mut stage = Stage::new(true);
    let (camera, subject, neighbour) = (stage.camera, stage.subject, stage.neighbour);
    stage
        .then(Backend::Cpu, |_scene| {})
        .assert_retained("lit CPU scene, nothing changed");
    stage
        .then(Backend::Cpu, tint(subject))
        .assert_retained("lit CPU scene, tinted box");
    // Per-vertex shading was computed with the direction to the camera.
    stage
        .then(Backend::Cpu, orbit_to(camera, 75.0))
        .assert_full_prepare(Reason::ViewDependentBake, "lit CPU scene, camera orbited");
    // Shadows cast by the neighbour are part of that shading.
    stage
        .then(Backend::Cpu, set_visible(neighbour, false))
        .assert_full_prepare(
            Reason::PlacementDependentBake,
            "lit CPU scene, shadow caster hidden",
        );
}

#[test]
fn the_cpu_renderer_bakes_moved_content_where_it_is() {
    let mut stage = Stage::new(false);
    let subject = stage.subject;
    stage
        .then(Backend::Cpu, move_to(subject, Vec3::new(-0.3, 0.2, 0.1)))
        .assert_full_prepare(Reason::CpuPlacement, "unlit CPU scene, box moved");
}

#[test]
fn hiding_and_showing_a_node_again_restores_it_from_the_template() {
    for backend in BACKENDS {
        let mut stage = Stage::for_backend(backend);
        let (camera, neighbour) = (stage.camera, stage.neighbour);
        // One renderer sees the node, loses it and gets it back.
        let Some(mut renderer) = backend.renderer(SIZE, SIZE) else {
            return;
        };
        prepare_and_render(&mut renderer, &mut stage.scene, &stage.assets, camera);
        for visible in [false, true] {
            set_visible(neighbour, visible)(&mut stage.scene);
            let (frame, work) =
                prepare_and_render(&mut renderer, &mut stage.scene, &stage.assets, camera);
            assert_eq!(
                work,
                PrepareWork::RETAINED,
                "{backend:?}: neighbour visible={visible} must come from the template"
            );
            let reference = fresh_full_prepare_frame(
                backend,
                SIZE,
                SIZE,
                &mut stage.scene,
                &stage.assets,
                camera,
            );
            assert_eq!(
                frame_difference(&frame, &reference),
                (0, 0),
                "{backend:?}: neighbour visible={visible} must equal a full prepare"
            );
        }
    }
}

#[test]
fn reframing_the_camera_reuses_the_template() {
    for backend in BACKENDS {
        let mut stage = Stage::for_backend(backend);
        let camera = stage.camera;
        let projection_before = stage.scene.camera(camera).cloned();
        let Some(reframed) = stage.after(backend, |scene| {
            scene
                .frame(
                    camera,
                    Aabb::new(Vec3::new(-3.0, -1.0, -2.0), Vec3::new(3.0, 1.0, 2.0)),
                )
                .expect("camera frames the bounds");
        }) else {
            return;
        };
        assert_ne!(
            stage.scene.camera(camera).cloned(),
            projection_before,
            "framing must change the projection itself for this case to cover it"
        );
        reframed.assert_retained(&format!("{backend:?}: camera reframed"));
    }
}

#[test]
fn switching_the_active_camera_re_culls_and_keeps_the_template() {
    for backend in BACKENDS {
        let mut stage = Stage::for_backend(backend);
        let root = stage.scene.root();
        let second = stage
            .scene
            .add_perspective_camera(
                root,
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(1.7, 0.9, -1.5)).looking_at(Vec3::ZERO, Vec3::Y),
            )
            .expect("second camera inserts");
        let first = stage.camera;
        let Some(switched) = stage.after(backend, |scene| {
            scene.set_active_camera(second).expect("camera switches");
        }) else {
            return;
        };
        switched.assert_retained(&format!("{backend:?}: second camera made active"));

        // The second camera does not see the neighbour's layer.
        stage
            .scene
            .set_layer_mask(stage.neighbour, 0b10)
            .expect("neighbour moves to its own layer");
        stage
            .scene
            .set_camera_layer_mask(second, 0b01)
            .expect("second camera drops that layer");
        stage
            .then(backend, |scene| {
                scene.set_active_camera(first).expect("camera switches");
            })
            .assert_full_prepare(
                Reason::TemplateCoverage,
                &format!("{backend:?}: first camera sees a node the template never held"),
            );
        stage
            .then(backend, |scene| {
                scene.set_active_camera(second).expect("camera switches");
            })
            .assert_retained(&format!(
                "{backend:?}: second camera drops a node the template holds"
            ));
    }
}

#[test]
fn the_cpu_renderer_bakes_instance_sets_per_instance() {
    let mut stage = Stage::new(false);
    let camera = stage.camera;
    let root = stage.scene.root();
    let (_node, set) = stage
        .scene
        .add_instance_set_node(
            root,
            stage.box_geometry,
            stage.material,
            Transform::IDENTITY,
        )
        .expect("instance set inserts");
    for x in [-0.9, 0.9] {
        stage
            .scene
            .push_instance(set, Transform::at(Vec3::new(x, -0.6, 0.0)))
            .expect("instance inserts");
    }
    stage
        .then(Backend::Cpu, orbit_to(camera, 60.0))
        .assert_full_prepare(Reason::TemplateCoverage, "orbit around CPU instances");
}

#[test]
fn the_cpu_renderer_resolves_textures_for_the_view_they_are_seen_from() {
    let mut stage = Stage::new(false);
    let camera = stage.camera;
    let mut stripes = Vec::new();
    for y in 0..8_u8 {
        for _x in 0..8 {
            let level = if y % 2 == 0 { 255 } else { 40 };
            stripes.extend_from_slice(&[level, level, 255, 255]);
        }
    }
    let texture = stage
        .assets
        .create_texture(TextureMemoryDesc::rgba8_for_slot(
            TextureMemoryId::new("movement-tests/stripes").expect("identity is valid"),
            8,
            8,
            stripes,
            TextureSlot::BaseColor,
        ))
        .expect("stripe texture creates");
    let textured = stage
        .assets
        .create_material(MaterialDesc::unlit(Color::WHITE).with_base_color_texture(texture));
    stage
        .scene
        .mesh(stage.box_geometry, textured)
        .transform(Transform::at(Vec3::new(-0.8, 0.0, 0.0)))
        .add()
        .expect("textured box inserts");
    // The CPU renderer subdivides a textured triangle to its size on screen.
    stage
        .then(Backend::Cpu, orbit_to(camera, 60.0))
        .assert_full_prepare(Reason::ViewDependentBake, "orbit around a CPU-textured box");
}
