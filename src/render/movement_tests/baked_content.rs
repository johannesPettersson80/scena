//! Content that is baked for one view or one placement.
//!
//! A section box, reflection probes, skinning, LOD levels, particles, area
//! lights, labels, batched groups and tiled lights each fold something into
//! the template that a retained prepare cannot re-derive. Every case pins
//! which changes still keep the template, which bake again and why, and that
//! either way the frame is the one a renderer new to the scene produces.

use crate::SkinningMatrix;
use crate::geometry::{Aabb, GeometryDesc, GeometrySkin};
use crate::material::{Color, MaterialDesc};
use crate::scene::{
    AreaLight, LabelDesc, MeshLodLevel, Particle, ParticleSet, PerspectiveCamera, PointLight,
    ReflectionProbe, SceneSkinBinding, SectionBox, Transform, Vec3,
};

use super::super::prepare_lifecycle::FullPrepareReason as Reason;
use super::reuse_stage::{
    BACKENDS, RADIUS, RED, SIZE, Stage, move_to, orbit_to, set_visible, solid_environment, tint,
};
use super::support::{
    Backend, PrepareWork, frame_difference, fresh_full_prepare_frame, orbit_camera,
    prepare_and_render,
};

#[test]
fn a_section_box_needs_new_caps_only_when_content_moves() {
    for backend in BACKENDS {
        let mut stage = Stage::for_backend(backend);
        let (camera, subject) = (stage.camera, stage.subject);
        stage
            .scene
            .set_section_box(SectionBox::from_bounds(Aabb::new(
                Vec3::new(-2.0, -2.0, -2.0),
                Vec3::new(0.1, 2.0, 2.0),
            )))
            .expect("section box applies");
        let Some(orbited) = stage.after(backend, orbit_to(camera, 115.0)) else {
            return;
        };
        orbited.assert_retained(&format!("{backend:?}: orbit around a sectioned box"));
        stage
            .then(backend, move_to(subject, Vec3::new(-0.2, 0.1, 0.0)))
            .assert_full_prepare(
                if backend == Backend::Cpu {
                    Reason::CpuPlacement
                } else {
                    Reason::SectionCaps
                },
                &format!("{backend:?}: sectioned box moved"),
            );
    }
}

/// Pixels that are clearly more red than green or blue.
///
/// A probe this bright tone-maps to a pale red, about `[253, 150, 150]`, so
/// the margin is a difference rather than a ratio. A ratio only matched the
/// darker, more saturated pixels that anti-aliasing leaves along the box's
/// outline, which made the count a measure of the anti-aliasing filter.
fn strongly_red_pixels(frame: &[u8]) -> usize {
    frame
        .chunks_exact(4)
        .filter(|pixel| {
            let clear_of = |other: u8| u16::from(pixel[0]) > u16::from(other) + 60;
            pixel[0] > 96 && clear_of(pixel[1]) && clear_of(pixel[2])
        })
        .count()
}

#[test]
fn reflection_probes_are_baked_again_when_they_change_or_content_moves() {
    let backend = Backend::Gpu;
    let mut stage = Stage::new(true);
    let (camera, subject, neighbour) = (stage.camera, stage.subject, stage.neighbour);
    stage.environment = Some(solid_environment(
        &stage.assets,
        "global-grey",
        [0.2, 0.2, 0.2],
    ));
    let red = solid_environment(&stage.assets, "probe-red", [4.0, 0.03, 0.02]);
    let blue = solid_environment(&stage.assets, "probe-blue", [0.02, 0.05, 4.0]);
    let metal = stage
        .assets
        .create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 1.0, 0.1));
    let mirror = stage
        .scene
        .mesh(stage.box_geometry, metal)
        .transform(Transform::at(Vec3::new(-0.8, 0.0, 0.0)))
        .add()
        .expect("mirror box inserts");
    let probe = stage
        .scene
        .add_reflection_probe(
            ReflectionProbe::new(Aabb::new(
                Vec3::new(-1.4, -0.6, -0.6),
                Vec3::new(-0.3, 0.6, 0.6),
            ))
            .with_resolution(16)
            .with_environment(red)
            .assign_node(mirror),
        )
        .expect("probe inserts");

    let Some(orbited) = stage.after(backend, orbit_to(camera, 300.0)) else {
        return;
    };
    orbited.assert_retained("orbit around a probe-lit box");
    let red_before = strongly_red_pixels(&orbited.frame);
    assert!(
        red_before > 40,
        "the mirror box must show its red probe for this test to cover probes at all, \
         but only {red_before} pixels are red"
    );

    // The scene reports this through the same revision as the tint below.
    let replaced = stage.then(backend, |scene| {
        scene
            .set_reflection_probe_environment(probe, blue)
            .expect("probe takes another environment");
    });
    replaced.assert_full_prepare(Reason::ReflectionProbes, "probe environment replaced");
    let red_after = strongly_red_pixels(&replaced.frame);
    assert!(
        red_after * 4 < red_before,
        "the mirror box must stop showing the replaced probe: {red_after} of {red_before} \
         red pixels remain"
    );

    stage
        .then(backend, tint(neighbour))
        .assert_retained("tint beside a probe-lit box");
    // A mesh is lit by the probe whose bounds it sits in.
    stage
        .then(backend, move_to(subject, Vec3::new(0.1, 0.3, 0.0)))
        .assert_full_prepare(
            Reason::ReflectionProbes,
            "box moved in a scene with a probe",
        );
    stage
        .then(backend, |scene| {
            scene
                .remove_reflection_probe(probe)
                .expect("probe is removed");
        })
        .assert_full_prepare(Reason::ReflectionProbes, "probe removed");
}

#[test]
fn a_skinned_mesh_is_only_deformed_again_when_content_moves() {
    for backend in BACKENDS {
        let mut stage = Stage::for_backend(backend);
        let camera = stage.camera;
        let skinned_geometry = stage.assets.create_geometry(
            GeometryDesc::box_xyz(0.3, 0.5, 0.3)
                .with_skin(GeometrySkin::new(
                    vec![[0, 0, 0, 0]; 24],
                    vec![[1.0, 0.0, 0.0, 0.0]; 24],
                ))
                .expect("skinned geometry builds"),
        );
        let root = stage.scene.root();
        let joint = stage
            .scene
            .add_empty(root, Transform::at(Vec3::new(-0.7, 0.2, 0.0)))
            .expect("joint inserts");
        let skinned = stage
            .scene
            .mesh(skinned_geometry, stage.material)
            .add()
            .expect("skinned mesh inserts");
        stage
            .scene
            .set_skin_binding(
                skinned,
                SceneSkinBinding::new(vec![joint], vec![SkinningMatrix::IDENTITY]),
            )
            .expect("skin binding applies");

        let Some(orbited) = stage.after(backend, orbit_to(camera, 250.0)) else {
            return;
        };
        orbited.assert_retained(&format!("{backend:?}: orbit around a skinned mesh"));
        stage
            .then(backend, move_to(joint, Vec3::new(-0.7, -0.2, 0.3)))
            .assert_full_prepare(
                if backend == Backend::Cpu {
                    Reason::CpuPlacement
                } else {
                    Reason::Skinning
                },
                &format!("{backend:?}: joint moved"),
            );
    }
}

#[test]
fn a_joint_moved_while_its_mesh_was_hidden_deforms_the_mesh_once_it_is_shown() {
    let backend = Backend::Gpu;
    let mut stage = Stage::new(true);
    let camera = stage.camera;
    let skinned_geometry = stage.assets.create_geometry(
        GeometryDesc::box_xyz(0.3, 0.5, 0.3)
            .with_skin(GeometrySkin::new(
                vec![[0, 0, 0, 0]; 24],
                vec![[1.0, 0.0, 0.0, 0.0]; 24],
            ))
            .expect("skinned geometry builds"),
    );
    let root = stage.scene.root();
    let joint = stage
        .scene
        .add_empty(root, Transform::at(Vec3::new(-0.7, 0.2, 0.0)))
        .expect("joint inserts");
    let skinned = stage
        .scene
        .mesh(skinned_geometry, stage.material)
        .add()
        .expect("skinned mesh inserts");
    stage
        .scene
        .set_skin_binding(
            skinned,
            SceneSkinBinding::new(vec![joint], vec![SkinningMatrix::IDENTITY]),
        )
        .expect("skin binding applies");

    let Some(mut renderer) = backend.renderer(SIZE, SIZE) else {
        return;
    };
    prepare_and_render(&mut renderer, &mut stage.scene, &stage.assets, camera);
    set_visible(skinned, false)(&mut stage.scene);
    prepare_and_render(&mut renderer, &mut stage.scene, &stage.assets, camera);
    // No skinned mesh is visible while its joint moves.
    move_to(joint, Vec3::new(-0.7, -0.3, 0.3))(&mut stage.scene);
    prepare_and_render(&mut renderer, &mut stage.scene, &stage.assets, camera);
    set_visible(skinned, true)(&mut stage.scene);
    let (frame, _work) = prepare_and_render(&mut renderer, &mut stage.scene, &stage.assets, camera);

    let reference =
        fresh_full_prepare_frame(backend, SIZE, SIZE, &mut stage.scene, &stage.assets, camera);
    assert_eq!(
        frame_difference(&frame, &reference),
        (0, 0),
        "the shown mesh must be deformed by where its joint is now"
    );
}

#[test]
fn a_mesh_with_lod_levels_is_baked_again_only_when_another_level_is_selected() {
    for backend in BACKENDS {
        let mut stage = Stage::for_backend(backend);
        let (camera, subject) = (stage.camera, stage.subject);
        let coarse = stage
            .assets
            .create_geometry(GeometryDesc::box_xyz(0.5, 0.7, 0.5));
        stage
            .scene
            .set_mesh_lods(subject, vec![MeshLodLevel::new(0.15, coarse)])
            .expect("LOD levels apply");

        let Some(orbited) = stage.after(backend, orbit_to(camera, 160.0)) else {
            return;
        };
        orbited.assert_retained(&format!(
            "{backend:?}: orbit at a distance that keeps the level"
        ));
        stage
            .then(backend, |scene| {
                orbit_camera(scene, camera, 160.0, 12.0);
            })
            .assert_full_prepare(
                Reason::MeshLod,
                &format!("{backend:?}: camera pulled back to the coarse level"),
            );
        // Switching cameras bumps no scene revision at all, yet the camera
        // that is active now selects the fine level again.
        let root = stage.scene.root();
        let near = stage
            .scene
            .add_perspective_camera(
                root,
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(0.0, 0.0, RADIUS)),
            )
            .expect("near camera inserts");
        stage
            .then(backend, |scene| {
                scene.set_active_camera(near).expect("camera switches");
            })
            .assert_full_prepare(
                Reason::MeshLod,
                &format!("{backend:?}: switched to a camera that selects the fine level"),
            );
    }
}

#[test]
fn particles_face_the_camera_they_were_baked_for() {
    for backend in BACKENDS {
        let mut stage = Stage::for_backend(backend);
        let (camera, subject) = (stage.camera, stage.subject);
        let root = stage.scene.root();
        stage
            .scene
            .add_particle_set(
                root,
                ParticleSet::try_new(vec![
                    Particle::new(Vec3::new(-0.6, 0.5, 0.2), Color::WHITE, 9.0),
                    Particle::new(Vec3::new(0.2, -0.6, 0.5), RED, 13.0),
                ])
                .expect("particle set is valid"),
                Transform::at(Vec3::new(0.0, 0.1, 0.0)),
            )
            .expect("particles insert");

        let Some(tinted) = stage.after(backend, tint(subject)) else {
            return;
        };
        tinted.assert_retained(&format!("{backend:?}: tint beside particles"));
        stage
            .then(backend, orbit_to(camera, 200.0))
            .assert_full_prepare(
                Reason::ViewDependentBake,
                &format!("{backend:?}: orbit around particles"),
            );
    }
}

#[test]
fn area_light_visibility_is_traced_again_when_the_view_or_an_occluder_changes() {
    for backend in BACKENDS {
        let mut stage = Stage::new(true);
        let (camera, subject, neighbour) = (stage.camera, stage.subject, stage.neighbour);
        stage
            .scene
            .area_light(AreaLight::softbox())
            .transform(Transform::at(Vec3::new(0.4, 1.8, 0.6)).looking_at(Vec3::ZERO, Vec3::Z))
            .add()
            .expect("area light inserts");

        let Some(tinted) = stage.after(backend, tint(subject)) else {
            return;
        };
        tinted.assert_retained(&format!("{backend:?}: tint under an area light"));
        stage
            .then(backend, orbit_to(camera, 95.0))
            .assert_full_prepare(
                Reason::ViewDependentBake,
                &format!("{backend:?}: orbit under an area light"),
            );
        stage
            .then(backend, set_visible(neighbour, false))
            .assert_full_prepare(
                Reason::PlacementDependentBake,
                &format!("{backend:?}: occluder hidden under an area light"),
            );
    }
}

#[test]
fn labels_are_laid_out_again_for_another_view_or_another_set_of_labels() {
    for backend in BACKENDS {
        let mut stage = Stage::for_backend(backend);
        let (camera, subject) = (stage.camera, stage.subject);
        let root = stage.scene.root();
        let mut label = |text: &str, position: Vec3| {
            stage
                .scene
                .add_label_node(root, LabelDesc::new(text), Transform::at(position))
                .expect("label inserts")
                .1
        };
        label("A", Vec3::new(-0.5, 0.6, 0.0));
        let second = label("B", Vec3::new(0.5, -0.6, 0.0));

        let Some(tinted) = stage.after(backend, tint(subject)) else {
            return;
        };
        tinted.assert_retained(&format!("{backend:?}: tint beside labels"));
        stage
            .then(backend, orbit_to(camera, 320.0))
            .assert_full_prepare(Reason::Labels, &format!("{backend:?}: orbit with labels"));
        stage
            .then(backend, set_visible(second, false))
            .assert_full_prepare(Reason::Labels, &format!("{backend:?}: label hidden"));
    }
}

#[test]
fn an_automatically_batched_group_follows_its_member_nodes() {
    let backend = Backend::Gpu;
    let mut stage = Stage::new(true);
    let camera = stage.camera;
    let members = [-0.9_f32, -0.3, 0.3, 0.9, 1.5].map(|x| {
        stage
            .scene
            .mesh(stage.box_geometry, stage.material)
            .transform(Transform::at(Vec3::new(x * 0.5, -0.7, x * 0.3)).rotate_y_deg(x * 40.0))
            .add()
            .expect("group member inserts")
    });

    let Some(orbited) = stage.after(backend, orbit_to(camera, 130.0)) else {
        return;
    };
    orbited.assert_retained("orbit around a batched group");
    stage
        .then(backend, tint(members[1]))
        .assert_retained("member of a batched group tinted");
    let moved = stage.then(backend, move_to(members[3], Vec3::new(0.2, 0.8, -0.4)));
    assert_eq!(
        (moved.work, moved.full_prepare_reason),
        (PrepareWork::RETAINED, None),
        "member of a batched group moved"
    );
    moved.assert_matches_full_prepare_within_rounding("member of a batched group moved");
    // Five members stay visible: still one group, minus one record.
    stage
        .then(backend, set_visible(members[0], false))
        .assert_retained("member of a batched group hidden");
    // The template is baked, clipped and attributed as the group's first
    // member, which is the stage's own box.
    let subject = stage.subject;
    stage
        .then(backend, set_visible(subject, false))
        .assert_full_prepare(
            Reason::TemplateCoverage,
            "first member of a batched group hidden",
        );
    // Four members are left, and three are drawn one by one.
    stage
        .then(backend, set_visible(members[2], false))
        .assert_full_prepare(
            Reason::TemplateCoverage,
            "batched group shrunk below the size that is batched",
        );
}

#[test]
fn tiled_lights_are_binned_again_for_another_view() {
    let backend = Backend::Gpu;
    let mut stage = Stage::new(true);
    let (camera, subject) = (stage.camera, stage.subject);
    for index in 0..18 {
        let angle = index as f32 * 0.35;
        stage
            .scene
            .point_light(PointLight::default().with_intensity_candela(4.0))
            .transform(Transform::at(Vec3::new(
                1.2 * angle.cos(),
                0.6,
                1.2 * angle.sin(),
            )))
            .add()
            .expect("point light inserts");
    }

    let Some(tinted) = stage.after(backend, tint(subject)) else {
        return;
    };
    tinted.assert_retained("tint under tiled lights");
    stage
        .then(backend, orbit_to(camera, 215.0))
        .assert_full_prepare(Reason::TiledLights, "orbit under tiled lights");
}
