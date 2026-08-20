use super::{PHOTOGRAPHIC_BRIGHT_CARD_LINEAR_RADIANCE, SceneHostCore, SceneHostError};
use crate::{AssetFetcher, Color, GeometryDesc, MaterialDesc, NodeKey, Transform, Vec3};

#[derive(Debug, Clone, Copy)]
pub(super) struct PhotographicReflectionCardSpec {
    #[cfg(test)]
    pub(super) role: &'static str,
    position: Vec3,
    #[cfg(test)]
    pub(super) width_m: f32,
    #[cfg(test)]
    pub(super) height_m: f32,
    #[cfg(test)]
    pub(super) distance_from_subject_m: f32,
    #[cfg(test)]
    pub(super) angle_from_camera_axis_degrees: f32,
    #[cfg(test)]
    pub(super) linear_color: [f32; 3],
    #[cfg(test)]
    pub(super) emissive_strength: f32,
}

#[derive(Debug)]
pub(super) struct PhotographicReflectionCards {
    nodes: [NodeKey; 2],
    #[cfg(test)]
    pub(super) specs: [PhotographicReflectionCardSpec; 2],
    #[cfg(test)]
    pub(super) subject_extent_m: Vec3,
    #[cfg(test)]
    pub(super) subject_radius_m: f32,
}

#[derive(Debug, Clone, Copy)]
struct ReflectionCardViewBasis {
    right: Vec3,
    front: Vec3,
}

impl ReflectionCardViewBasis {
    fn from_camera(subject: Vec3, camera: Vec3) -> Self {
        let front = (camera - subject).normalize_or_zero();
        let front = if front.length_squared() > 1.0e-8 {
            front
        } else {
            Vec3::Z
        };
        let right = Vec3::Y.cross(front).normalize_or_zero();
        let right = if right.length_squared() > 1.0e-8 {
            right
        } else {
            Vec3::X
        };
        Self { right, front }
    }
}

pub(super) fn install_photographic_reflection_cards<F: AssetFetcher>(
    host: &mut SceneHostCore<F>,
    subject: NodeKey,
) -> Result<PhotographicReflectionCards, SceneHostError> {
    let bounds = host
        .scene
        .node_world_bounds(subject, &host.assets)?
        .ok_or(crate::LookupError::ImportHasNoBounds)?;
    let center = bounds.center();
    let extent = bounds.half_extent() * 2.0;
    let radius = bounds.bounding_sphere_radius().max(0.05);
    let camera_position = host
        .scene
        .camera_node(host.active_camera)
        .and_then(|node| host.scene.world_transform(node))
        .map(|transform| transform.translation)
        .unwrap_or(center + Vec3::Z * radius * 4.0);
    let view = ReflectionCardViewBasis::from_camera(center, camera_position);
    let angle_degrees = 40.0_f32;
    let angle = angle_degrees.to_radians();
    let distance = radius * 2.0;
    let height = extent.y * 2.0;
    let width = extent.x.max(extent.z) * 2.0;
    let forward = distance * angle.cos();
    let lateral = distance * angle.sin();
    let specs = [
        PhotographicReflectionCardSpec {
            #[cfg(test)]
            role: "bright_strip",
            position: center + view.front * forward - view.right * lateral,
            #[cfg(test)]
            width_m: width,
            #[cfg(test)]
            height_m: height,
            #[cfg(test)]
            distance_from_subject_m: distance,
            #[cfg(test)]
            angle_from_camera_axis_degrees: angle_degrees,
            #[cfg(test)]
            linear_color: [1.0, 1.0, 1.0],
            #[cfg(test)]
            emissive_strength: PHOTOGRAPHIC_BRIGHT_CARD_LINEAR_RADIANCE,
        },
        PhotographicReflectionCardSpec {
            #[cfg(test)]
            role: "dark_flag",
            position: center + view.front * forward + view.right * lateral,
            #[cfg(test)]
            width_m: width,
            #[cfg(test)]
            height_m: height,
            #[cfg(test)]
            distance_from_subject_m: distance,
            #[cfg(test)]
            angle_from_camera_axis_degrees: angle_degrees,
            #[cfg(test)]
            linear_color: [0.03, 0.03, 0.03],
            #[cfg(test)]
            emissive_strength: 0.0,
        },
    ];
    let geometry = host
        .assets
        .create_geometry(GeometryDesc::box_xyz(width, height, radius * 0.01));
    let bright_material = host.assets.create_material(
        MaterialDesc::unlit(Color::BLACK)
            .with_emissive(Color::from_linear_rgb(1.0, 1.0, 1.0))
            .with_emissive_strength(PHOTOGRAPHIC_BRIGHT_CARD_LINEAR_RADIANCE),
    );
    let dark_material = host
        .assets
        .create_material(MaterialDesc::unlit(Color::from_linear_rgb(
            0.03, 0.03, 0.03,
        )));
    let mut nodes = Vec::with_capacity(2);
    for (spec, material) in specs.iter().zip([bright_material, dark_material]) {
        nodes.push(
            host.scene
                .mesh(geometry, material)
                .transform(Transform::at(spec.position).looking_at(center, Vec3::Y))
                .add()?,
        );
    }
    Ok(PhotographicReflectionCards {
        nodes: [nodes[0], nodes[1]],
        #[cfg(test)]
        specs,
        #[cfg(test)]
        subject_extent_m: extent,
        #[cfg(test)]
        subject_radius_m: radius,
    })
}

pub(super) fn remove_photographic_reflection_cards<F: AssetFetcher>(
    host: &mut SceneHostCore<F>,
    cards: &PhotographicReflectionCards,
) -> Result<(), SceneHostError> {
    for node in cards.nodes {
        if host.scene.visible(node).is_some() {
            host.scene.remove_node(node)?;
        }
    }
    Ok(())
}
