use crate::assets::Assets;
use crate::geometry::GeometryDesc;
use crate::material::{Color, MaterialDesc};
use crate::scene::{Scene, Transform, Vec3};

use crate::render::Renderer;

/// Area-light shadow visibility is ray traced per baked vertex against a BVH and
/// dominates prepare cost for any scene that has area lights. The camera-behavior
/// loop prepares repeatedly while changing only the camera or the exposure, so
/// the populated cache has to survive a prepare or that work is redone every
/// time. Measured on the demo path, carrying it across prepares took a 1280x840
/// camera-behavior render from 205 s to 61 s.
///
/// Cache entries are keyed by the lighting and occluder signatures as well, so
/// reuse cannot return a stale value; the second half of this test pins that
/// changed lighting really does re-trace.
#[test]
fn area_shadow_visibility_survives_a_prepare_and_is_dropped_when_lighting_changes() {
    use crate::scene::AreaLight;

    let mut renderer = Renderer::headless(64, 64).expect("CPU renderer builds");
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.8, 0.8));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(200, 200, 200),
        0.0,
        0.5,
    ));

    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("mesh adds");
    scene
        .area_light(AreaLight::softbox())
        .transform(Transform::at(Vec3::new(0.0, 2.0, 0.0)))
        .add()
        .expect("area light adds");

    let first = renderer
        .prepare_with_assets_profiled(&mut scene, &assets)
        .expect("first prepare succeeds");
    assert!(
        first.shadow_rays > 0,
        "an area-lit scene must trace shadow rays on the first prepare: {first:?}"
    );

    let second = renderer
        .prepare_with_assets_profiled(&mut scene, &assets)
        .expect("second prepare succeeds");
    assert!(
        second.shadow_rays * 2 < first.shadow_rays,
        "an unchanged scene must reuse baked area-shadow visibility instead of \
         re-tracing it, first={} second={}",
        first.shadow_rays,
        second.shadow_rays
    );

    scene
        .area_light(AreaLight::softbox())
        .transform(Transform::at(Vec3::new(1.5, 2.0, 0.0)))
        .add()
        .expect("second area light adds");
    let third = renderer
        .prepare_with_assets_profiled(&mut scene, &assets)
        .expect("third prepare succeeds");
    assert!(
        third.shadow_rays > second.shadow_rays,
        "changed lighting must invalidate baked area-shadow visibility, \
         second={} third={}",
        second.shadow_rays,
        third.shadow_rays
    );
}
