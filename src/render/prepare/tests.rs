use super::*;
use crate::assets::Assets;
use crate::diagnostics::Backend;
use crate::geometry::GeometryDesc;
use crate::material::{Color, MaterialDesc};
use crate::render::prepare::cpu_bake::baked_shadow_visibility;
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

#[test]
fn blended_material_primitives_skip_gpu_depth_prepass() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::clear_glass(Color::CYAN));
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let material_slots = collect_backend_material_slots(&scene, Some(&assets));
    let material_handles = material_slots
        .iter()
        .map(|slot| slot.handle)
        .collect::<Vec<_>>();

    let prepared = collect_prepared_primitives(
        RasterTarget {
            width: 64,
            height: 64,
            backend: Backend::WebGl2,
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
            .any(|primitive| !primitive.depth_prepass_eligible()),
        "alpha-blended material primitives must not write the GPU depth pre-pass; \
         otherwise glass occludes the scene it is supposed to transmit or blend over"
    );
    assert!(
        prepared
            .primitives
            .iter()
            .all(|primitive| !primitive.depth_prepass_eligible()),
        "all primitives in this one-glass-mesh scene should be transparent pass primitives"
    );
}

#[test]
fn opaque_node_tint_is_retained_per_primitive_not_baked_into_vertex_colors() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let mesh = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-1.5, 0.0, 0.0)))
        .add()
        .expect("mesh inserts");
    let instance_set = scene
        .add_instance_set(
            scene.root(),
            geometry,
            material,
            Transform::at(Vec3::new(1.5, 0.0, 0.0)),
        )
        .expect("instance set inserts");
    scene
        .push_instance(instance_set, Transform::IDENTITY)
        .expect("instance inserts");
    let (instance_node, _, _) = scene
        .instance_set_nodes()
        .next()
        .expect("instance-set node is visible");

    scene
        .set_node_tint(mesh, Some(Color::from_linear_rgba(1.0, 0.0, 0.0, 1.0)))
        .expect("mesh tint sets");
    scene
        .set_node_tint(
            instance_node,
            Some(Color::from_linear_rgba(0.0, 0.0, 1.0, 1.0)),
        )
        .expect("instance-set tint sets");

    let prepared = collect_prepared_primitives(
        RasterTarget {
            width: 64,
            height: 64,
            backend: Backend::Headless,
        },
        &scene,
        Some(&assets),
        None,
        &[],
        &[],
        PreparedEnvironmentLighting::default(),
    )
    .expect("scene prepares");

    assert!(
        prepared.primitives.iter().any(|primitive| {
            primitive.tint() == Color::from_linear_rgba(1.0, 0.0, 0.0, 1.0)
                && primitive
                    .vertices()
                    .iter()
                    .all(|vertex| vertex.color == Color::WHITE)
        }),
        "mesh tint should be retained as draw tint without rewriting vertex colors"
    );
    assert!(
        prepared.primitives.iter().any(|primitive| {
            primitive.tint() == Color::from_linear_rgba(0.0, 0.0, 1.0, 1.0)
                && primitive
                    .vertices()
                    .iter()
                    .all(|vertex| vertex.color == Color::WHITE)
        }),
        "instance-set tint should be retained as draw tint without rewriting vertex colors"
    );
}
