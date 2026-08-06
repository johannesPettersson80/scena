use super::*;
use crate::assets::Assets;
use crate::diagnostics::Backend;
use crate::geometry::GeometryDesc;
use crate::material::{Color, MaterialDesc};
use crate::render::prepare::cpu_bake::baked_shadow_visibility;
use crate::scene::{ReflectionProbe, Transform};

#[test]
fn backend_shaded_materials_skip_cpu_shadow_visibility_bake() {
    let scene = Scene::new();
    let lights = PreparedLights::from_scene(&scene, Vec3::ZERO);
    let position = Vec3::new(0.0, 0.0, 0.0);
    let occluders = shadows::ShadowOccluderSet::default();
    let cache = shadows::ShadowVisibilityCache::new(&lights, &occluders);

    assert_eq!(
        baked_shadow_visibility(position, &lights, &occluders, &cache, true),
        1.0
    );
    assert_eq!(
        baked_shadow_visibility(position, &lights, &occluders, &cache, false),
        1.0
    );
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
        1.0,
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
        1.0,
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
        1.0,
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

#[test]
fn hidden_instances_are_filtered_from_gpu_instance_records() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let instance_set = scene
        .add_instance_set(scene.root(), geometry, material, Transform::IDENTITY)
        .expect("instance set inserts");
    let visible = scene
        .push_instance(instance_set, Transform::at(Vec3::new(-1.0, 0.0, 0.0)))
        .expect("visible instance inserts");
    let hidden = scene
        .push_instance(instance_set, Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .expect("hidden instance inserts");
    scene
        .set_instance_visible(instance_set, hidden, false)
        .expect("hidden instance visibility sets");

    let prepared = collect_prepared_primitives(
        RasterTarget {
            width: 64,
            height: 64,
            backend: Backend::HeadlessGpu,
        },
        1.0,
        &scene,
        Some(&assets),
        None,
        &[],
        &[],
        PreparedEnvironmentLighting::default(),
    )
    .expect("scene prepares");

    assert_eq!(prepared.instances.len(), 1);
    let records = prepared.instances[0].instances();
    assert_eq!(
        records.len(),
        1,
        "hidden instances must not be retained for GPU instanced draws"
    );
    assert_eq!(records[0].source_instance(), Some(visible));
}

#[test]
fn repeated_mesh_nodes_auto_instance_on_gpu_prepare_path() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    for x in [-1.5, -0.5, 0.5, 1.5] {
        scene
            .mesh(geometry, material)
            .transform(Transform::at(Vec3::new(x, 0.0, 0.0)))
            .add()
            .expect("mesh inserts");
    }

    let prepared = collect_prepared_primitives(
        RasterTarget {
            width: 64,
            height: 64,
            backend: Backend::HeadlessGpu,
        },
        1.0,
        &scene,
        Some(&assets),
        None,
        &[],
        &[],
        PreparedEnvironmentLighting::default(),
    )
    .expect("scene prepares");

    assert_eq!(
        prepared.instances.len(),
        1,
        "larger repeated ordinary mesh groups with the same geometry/material should auto-instance on the GPU path"
    );
    assert_eq!(
        prepared.instances[0].instances().len(),
        4,
        "the auto-instance batch must preserve every repeated node transform as an instance record"
    );
    assert!(
        prepared.primitives.is_empty(),
        "auto-instanced repeated mesh nodes should not also emit duplicate ordinary primitives"
    );

    let cpu_prepared = collect_prepared_primitives(
        RasterTarget {
            width: 64,
            height: 64,
            backend: Backend::Headless,
        },
        1.0,
        &scene,
        Some(&assets),
        None,
        &[],
        &[],
        PreparedEnvironmentLighting::default(),
    )
    .expect("cpu scene prepares");
    assert!(
        cpu_prepared.instances.is_empty(),
        "CPU reference rendering stays on ordinary primitives; auto-instancing is a GPU draw-call optimization"
    );
}

#[test]
fn screen_space_strokes_scale_to_full_frame_supersample_pixels() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::line(
        Vec3::new(-0.5, 0.0, 0.0),
        Vec3::new(0.5, 0.0, 0.0),
    ));
    let material = assets.create_material(MaterialDesc::line(Color::WHITE, 2.0));
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("line inserts");

    let prepared = collect_prepared_primitives(
        RasterTarget {
            width: 128,
            height: 128,
            backend: Backend::HeadlessGpu,
        },
        2.0,
        &scene,
        Some(&assets),
        None,
        &[],
        &[],
        PreparedEnvironmentLighting::default(),
    )
    .expect("line scene prepares");

    assert_eq!(prepared.strokes.len(), 1);
    assert_eq!(
        prepared.strokes[0].width_px(),
        4.0,
        "screen-space stroke widths are authored in final-frame pixels and must be expanded when drawing into a supersampled internal target"
    );
}

#[test]
fn assigned_mesh_primitives_retain_their_local_reflection_probe() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 1.0, 0.2));
    let probe_environment = assets.create_environment(
        crate::assets::EnvironmentDesc::from_cubemap_radiance(
            "scena://generated/reflection-probe/test",
            2,
            std::array::from_fn(|face| vec![[face as f32 + 1.0; 3]; 4]),
        )
        .expect("probe cubemap is valid"),
    );
    let mut scene = Scene::new();
    let assigned = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-1.0, 0.0, 0.0)))
        .add()
        .expect("assigned mesh inserts");
    let unassigned = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .add()
        .expect("unassigned mesh inserts");
    scene
        .add_reflection_probe(
            ReflectionProbe::new(crate::geometry::Aabb::new(
                Vec3::new(-2.0, -1.0, -1.0),
                Vec3::new(0.0, 1.0, 1.0),
            ))
            .with_environment(probe_environment)
            .assign_node(assigned),
        )
        .expect("probe inserts");
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
        1.0,
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
            .filter(|primitive| primitive.source_node() == Some(assigned))
            .all(|primitive| primitive.reflection_probe().is_some()),
        "every triangle of the assigned component must retain the selected probe",
    );
    assert!(
        prepared
            .primitives
            .iter()
            .filter(|primitive| primitive.source_node() == Some(unassigned))
            .all(|primitive| primitive.reflection_probe().is_none()),
        "unassigned geometry must fall back to the renderer's global environment",
    );
}
