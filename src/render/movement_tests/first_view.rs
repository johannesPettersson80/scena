//! GPU resources must not be shaped by what the first prepared camera saw.
//!
//! A full prepare uploads the whole scene and then draws the part the camera
//! sees. The retained path keeps those resources while the camera moves, so
//! anything a full prepare decides from the visible part alone goes stale the
//! moment other geometry comes into view. Two decisions were made that way:
//! whether to build the depth prepass at all (without it the colour pass has
//! no depth test), and whether to build the alpha-tested depth pipelines that
//! cut holes for masked materials.

use crate::assets::{Assets, TextureMemoryDesc, TextureMemoryId, TextureSlot};
use crate::geometry::GeometryDesc;
use crate::material::{AlphaMode, Color, MaterialDesc};
use crate::scene::{DirectionalLight, Scene, Transform, Vec3};

use super::support::{Backend, PrepareWork, retained_frame_after};

const SIZE: u32 = 64;

/// Keeps the camera at z = 2 and turns it about the vertical axis. At 180
/// degrees it looks away from the origin.
fn turn_camera(scene: &mut Scene, yaw_degrees: f32) {
    let camera = scene.active_camera().expect("active camera");
    let node = scene.camera_node(camera).expect("camera node resolves");
    scene
        .set_transform(
            node,
            Transform::at(Vec3::new(0.0, 0.0, 2.0)).rotate_y_deg(yaw_degrees),
        )
        .expect("camera turns");
}

fn lit_scene() -> (Scene, Assets) {
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    scene
        .directional_light(DirectionalLight::default())
        .add()
        .expect("light inserts");
    (scene, Assets::new())
}

#[test]
fn geometry_outside_the_first_view_is_depth_tested_once_it_comes_into_view() {
    let (mut scene, assets) = lit_scene();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let material =
        |color| assets.create_material(MaterialDesc::pbr_metallic_roughness(color, 0.0, 0.8));
    // The near box is inserted first, so without a depth test the far box
    // would be painted over it.
    scene
        .mesh(geometry, material(Color::from_linear_rgb(1.0, 0.1, 0.1)))
        .transform(Transform::at(Vec3::new(0.0, 0.0, 0.5)))
        .add()
        .expect("near box inserts");
    scene
        .mesh(geometry, material(Color::from_linear_rgb(0.1, 0.1, 1.0)))
        .transform(Transform::at(Vec3::new(0.2, 0.1, -0.5)))
        .add()
        .expect("far box inserts");
    turn_camera(&mut scene, 180.0);

    let Some(retained) = retained_frame_after(Backend::Gpu, SIZE, &mut scene, &assets, |scene| {
        turn_camera(scene, 0.0)
    }) else {
        return;
    };

    assert_eq!(
        retained.work,
        PrepareWork::RETAINED,
        "turning the camera towards the boxes is a camera-only change"
    );
    retained.assert_matches_full_prepare("boxes first prepared outside the view");
}

#[test]
fn masked_geometry_outside_the_first_view_is_cut_out_once_it_comes_into_view() {
    let (mut scene, assets) = lit_scene();
    let mut checker = Vec::new();
    for y in 0..8 {
        for x in 0..8 {
            let opaque = (x / 2 + y / 2) % 2 == 0;
            checker.extend_from_slice(&[255, 255, 255, if opaque { 255 } else { 0 }]);
        }
    }
    let texture = assets
        .create_texture(TextureMemoryDesc::rgba8_for_slot(
            TextureMemoryId::new("movement-tests/alpha-checker").expect("identity is valid"),
            8,
            8,
            checker,
            TextureSlot::BaseColor,
        ))
        .expect("checker texture creates");
    let masked = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_linear_rgb(1.0, 0.2, 0.2), 0.0, 0.8)
            .with_base_color_texture(texture)
            .with_alpha_mode(AlphaMode::Mask { cutoff: 0.5 })
            .with_double_sided(true),
    );
    let solid = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_linear_rgb(0.2, 0.2, 1.0),
        0.0,
        0.8,
    ));
    let plane = assets.create_geometry(GeometryDesc::plane(1.0, 1.0));
    let slab = assets.create_geometry(GeometryDesc::box_xyz(0.6, 0.6, 0.2));
    // A cut-out pane in front of a slab: the slab shows through the holes
    // only if the depth prepass alpha-tests the pane.
    scene
        .mesh(slab, solid)
        .transform(Transform::at(Vec3::new(0.0, 0.0, -0.6)))
        .add()
        .expect("slab inserts");
    scene
        .mesh(plane, masked)
        .transform(Transform::at(Vec3::new(0.0, 0.0, 0.3)).rotate_x_deg(90.0))
        .add()
        .expect("masked pane inserts");
    // Opaque geometry behind the camera keeps a depth prepass in the first
    // view, so this case isolates the alpha-test pipelines.
    scene
        .mesh(slab, solid)
        .transform(Transform::at(Vec3::new(0.0, 0.0, 5.0)))
        .add()
        .expect("witness slab inserts");
    turn_camera(&mut scene, 180.0);

    let Some(retained) = retained_frame_after(Backend::Gpu, SIZE, &mut scene, &assets, |scene| {
        turn_camera(scene, 0.0)
    }) else {
        return;
    };

    assert_eq!(retained.work, PrepareWork::RETAINED);
    retained.assert_matches_full_prepare("masked pane first prepared outside the view");
}
