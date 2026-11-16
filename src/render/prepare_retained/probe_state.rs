//! The reflection probes a template was baked with.

use crate::scene::{ReflectionProbe, ReflectionProbeKey, Scene};

/// Reflection probes as the scene held them at a full prepare.
///
/// Probe environments are uploaded with the template, and every mesh keeps the
/// probe it was assigned when it was baked. The scene reports a probe that was
/// added, removed or given another environment through its appearance
/// revision, the same counter an opaque node tint bumps. Comparing the probes
/// themselves is what lets a tint change keep the template while a probe
/// change does not.
#[derive(Debug, Clone, Default, PartialEq)]
pub(in crate::render) struct PreparedProbes {
    enabled: bool,
    probes: Vec<(ReflectionProbeKey, ReflectionProbe)>,
}

impl PreparedProbes {
    pub(in crate::render) fn of(scene: &Scene) -> Self {
        Self {
            enabled: scene.reflection_probes_enabled(),
            probes: scene
                .reflection_probes()
                .map(|(key, probe)| (key, probe.clone()))
                .collect(),
        }
    }

    /// Whether the scene still holds exactly the probes the template was
    /// baked with.
    pub(in crate::render) fn matches(&self, scene: &Scene) -> bool {
        let mut recorded = self.probes.iter();
        self.enabled == scene.reflection_probes_enabled()
            && scene.reflection_probes().all(|(key, probe)| {
                recorded.next().is_some_and(|(recorded_key, recorded)| {
                    *recorded_key == key && recorded == probe
                })
            })
            && recorded.next().is_none()
    }

    /// Whether a mesh can have been assigned one of these probes, which it is
    /// by where it sits in the world.
    pub(in crate::render) fn assign_by_placement(&self) -> bool {
        self.enabled && !self.probes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Assets;
    use crate::geometry::{Aabb, GeometryDesc};
    use crate::material::{Color, MaterialDesc};
    use crate::scene::Vec3;

    fn probe_over(node: crate::scene::NodeKey, half_extent: f32) -> ReflectionProbe {
        ReflectionProbe::new(Aabb::new(
            Vec3::new(-half_extent, -half_extent, -half_extent),
            Vec3::new(half_extent, half_extent, half_extent),
        ))
        .assign_node(node)
    }

    #[test]
    fn a_tint_keeps_the_probes_and_a_probe_change_does_not() {
        let assets = Assets::new();
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.2, 0.2, 0.2));
        let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let mut scene = Scene::new();
        let mesh = scene.mesh(geometry, material).add().expect("mesh inserts");

        let none = PreparedProbes::of(&scene);
        assert!(none.matches(&scene));
        assert!(
            !none.assign_by_placement(),
            "a scene without probes assigns none"
        );

        let first = scene
            .add_reflection_probe(probe_over(mesh, 1.0))
            .expect("probe inserts");
        assert!(
            !none.matches(&scene),
            "an added probe is not what the template was baked with"
        );

        let one = PreparedProbes::of(&scene);
        assert!(one.assign_by_placement());
        scene
            .set_node_tint(mesh, Some(Color::from_linear_rgb(1.0, 0.0, 0.0)))
            .expect("mesh tints");
        assert!(
            one.matches(&scene),
            "a tint bumps the appearance revision but leaves the probes alone"
        );

        scene
            .add_reflection_probe(probe_over(mesh, 2.0))
            .expect("second probe inserts");
        assert!(!one.matches(&scene), "a second probe changes the set");
        let two = PreparedProbes::of(&scene);
        scene
            .remove_reflection_probe(first)
            .expect("first probe removes");
        assert!(!two.matches(&scene), "a removed probe changes the set");
        assert!(
            !one.matches(&scene),
            "another probe in the place of the first is still another probe"
        );
    }
}
