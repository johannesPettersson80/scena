use super::*;
use crate::assets::Assets;
use crate::diagnostics::Backend;
use crate::material::Color;
use crate::scene::Transform;

#[test]
fn backend_shaded_materials_skip_cpu_shadow_visibility_bake() {
    let scene = Scene::new();
    let lights = PreparedLights::from_scene(&scene, Vec3::ZERO);
    let position = Vec3::new(0.0, 0.0, 0.0);

    assert_eq!(baked_shadow_visibility(position, &lights, &[], true), 1.0);
    assert_eq!(baked_shadow_visibility(position, &lights, &[], false), 1.0);
}

#[test]
fn asset_mesh_primitives_keep_model_draw_transform_for_gpu_templates() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8));
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .add()
        .expect("mesh inserts");
    let material_slots = collect_backend_material_slots(&scene, Some(&assets));
    let material_handles = material_slots
        .iter()
        .map(|slot| slot.handle)
        .collect::<Vec<_>>();

    let prepared = collect_prepared_primitives(
        RasterTarget {
            width: 64,
            height: 64,
            backend: Backend::HeadlessGpu,
        },
        &scene,
        Some(&assets),
        None,
        &[],
        &material_handles,
        PreparedEnvironmentLighting::default(),
    )
    .expect("scene prepares");

    assert!(
        prepared
            .primitives
            .iter()
            .any(|primitive| primitive.world_from_model()[12] == 2.0),
        "asset-backed GPU primitives must keep the model draw matrix so transform-only frames can update uniforms without rebuilding vertex bytes"
    );
}
