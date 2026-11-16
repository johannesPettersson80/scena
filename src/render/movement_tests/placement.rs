//! Where retained draws are, as the CPU side of the renderer sees them.
//!
//! The retained path re-places a draw by swapping its per-draw matrix. Frustum
//! culling and blend ordering read world-space corners on the CPU, so those
//! corners have to move with the matrix or the renderer culls and sorts the
//! scene as it stood at the last full prepare.

use crate::assets::Assets;
use crate::geometry::GeometryDesc;
use crate::material::{Color, MaterialDesc};
use crate::scene::{DirectionalLight, NodeKey, Scene, Transform, Vec3};

use super::support::{
    Backend, PrepareWork, fresh_full_prepare_frame, prepare_and_render, retained_frame_after,
    visible_centroid,
};

const SIZE: u32 = 64;

/// One box in view and one far outside the frustum.
fn two_box_scene() -> (Scene, Assets, NodeKey, NodeKey) {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.3, 0.3, 0.3));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_linear_rgb(0.9, 0.9, 0.9),
        0.0,
        0.8,
    ));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    scene
        .directional_light(DirectionalLight::default())
        .add()
        .expect("light inserts");
    let near = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-0.5, 0.0, 0.0)))
        .add()
        .expect("visible mesh inserts");
    let far = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(50.0, 0.0, 0.0)))
        .add()
        .expect("off-frustum mesh inserts");
    (scene, assets, near, far)
}

#[test]
fn a_mesh_moved_into_view_is_drawn_on_the_retained_gpu_path() {
    let (mut scene, assets, _near, far) = two_box_scene();
    let Some(retained) = retained_frame_after(Backend::Gpu, SIZE, &mut scene, &assets, |scene| {
        scene
            .set_transform(far, Transform::at(Vec3::new(0.5, 0.0, 0.0)))
            .expect("off-frustum mesh moves into view");
    }) else {
        return;
    };

    assert_eq!(retained.work, PrepareWork::RETAINED);
    let centroid = visible_centroid(&retained.frame, SIZE).expect("both boxes are visible");
    assert!(
        (centroid.0 - SIZE as f32 * 0.5).abs() <= 2.0,
        "the box that moved into view must be drawn next to the first one, centring the \
         visible pixels: {centroid:?}"
    );
    retained.assert_matches_full_prepare_within_rounding("mesh moved into view");
}

#[test]
fn retained_frustum_culling_counts_meshes_where_they_are_now() {
    let (mut scene, assets, near, far) = two_box_scene();
    let Some(mut renderer) = Backend::Gpu.renderer(SIZE, SIZE) else {
        return;
    };
    let camera = scene.active_camera().expect("active camera");
    prepare_and_render(&mut renderer, &mut scene, &assets, camera);
    let culled_at_start = renderer.stats().culled_objects;
    assert!(
        culled_at_start > 0,
        "the far box starts outside the frustum and must be culled"
    );

    // Swap the two: the visible box leaves, the culled one arrives.
    scene
        .set_transform(near, Transform::at(Vec3::new(-50.0, 0.0, 0.0)))
        .expect("visible mesh leaves");
    scene
        .set_transform(far, Transform::at(Vec3::new(0.5, 0.0, 0.0)))
        .expect("off-frustum mesh arrives");
    let (frame, work) = prepare_and_render(&mut renderer, &mut scene, &assets, camera);
    assert_eq!(work, PrepareWork::RETAINED);

    let mut reference = Backend::Gpu
        .renderer(SIZE, SIZE)
        .expect("the reference renderer uses the same backend");
    let (_, reference_work) = prepare_and_render(&mut reference, &mut scene, &assets, camera);
    assert_eq!(reference_work.full_prepares, 1);
    assert_eq!(
        renderer.stats().culled_objects,
        reference.stats().culled_objects,
        "retained frustum culling must cull the triangles a full prepare culls"
    );
    let centroid = visible_centroid(&frame, SIZE).expect("the arrived box is visible");
    assert!(
        centroid.0 > SIZE as f32 * 0.5,
        "only the box that arrived on the right may be drawn: {centroid:?}"
    );
}

#[test]
fn flattening_and_restoring_a_mesh_rebakes_instead_of_reusing_a_singular_placement() {
    // A zero scale has no inverse, so the vertices cannot be carried between
    // model and world space through it. Both directions need a full prepare.
    let (mut scene, assets, near, _far) = two_box_scene();
    let Some(mut renderer) = Backend::Gpu.renderer(SIZE, SIZE) else {
        return;
    };
    let camera = scene.active_camera().expect("active camera");
    prepare_and_render(&mut renderer, &mut scene, &assets, camera);

    for (case, scale) in [
        ("flattened", Vec3::new(1.0, 1.0, 0.0)),
        ("restored", Vec3::new(1.0, 1.5, 1.0)),
    ] {
        scene
            .set_transform(
                near,
                Transform::at(Vec3::new(-0.2, 0.0, 0.0))
                    .rotate_y_deg(30.0)
                    .with_scale(scale),
            )
            .expect("mesh rescales");
        let (frame, work) = prepare_and_render(&mut renderer, &mut scene, &assets, camera);
        assert_eq!(
            (work.full_prepares, work.retained_prepares),
            (1, 0),
            "{case}: a placement with no inverse cannot be reached or left by swapping matrices"
        );
        let reference =
            fresh_full_prepare_frame(Backend::Gpu, SIZE, SIZE, &mut scene, &assets, camera);
        assert_eq!(
            super::support::frame_difference(&frame, &reference),
            (0, 0),
            "{case}: the frame must equal a fresh full prepare"
        );
    }
}
