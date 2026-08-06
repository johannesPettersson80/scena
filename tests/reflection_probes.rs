use scena::{
    Aabb, Assets, Color, MaterialDesc, ReflectionProbe, ReflectionProbeError, Scene, Transform,
    Vec3,
};

#[test]
fn reflection_probes_require_assignment_and_select_smallest_containing_volume() {
    let assets = Assets::new();
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::LIGHT_GRAY,
        1.0,
        0.18,
    ));
    let mut scene = Scene::new();
    let component = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("component node inserts");

    let missing_assignment = scene
        .add_reflection_probe(ReflectionProbe::new(Aabb::new(
            Vec3::splat(-1.0),
            Vec3::splat(1.0),
        )))
        .expect_err("a probe with no component or material assignment is ambiguous");
    assert_eq!(missing_assignment, ReflectionProbeError::MissingAssignment);

    let broad = scene
        .add_reflection_probe(
            ReflectionProbe::new(Aabb::new(Vec3::splat(-2.0), Vec3::splat(2.0)))
                .assign_material(material),
        )
        .expect("broad probe inserts");
    let narrow = scene
        .add_reflection_probe(
            ReflectionProbe::new(Aabb::new(Vec3::splat(-0.5), Vec3::splat(0.5)))
                .assign_node(component)
                .assign_material(material),
        )
        .expect("narrow probe inserts");

    assert_eq!(scene.reflection_probe(broad).unwrap().resolution(), 256);
    assert_eq!(
        scene
            .select_reflection_probe(component, material, Vec3::ZERO)
            .map(|(key, _)| key),
        Some(narrow),
        "the smallest matching containing volume wins deterministically",
    );
    assert_eq!(
        scene
            .select_reflection_probe(component, material, Vec3::new(1.0, 0.0, 0.0))
            .map(|(key, _)| key),
        Some(broad),
    );
    assert!(
        scene
            .select_reflection_probe(component, material, Vec3::new(3.0, 0.0, 0.0))
            .is_none()
    );
}
