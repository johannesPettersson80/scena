use std::collections::HashSet;

use crate::assets::Assets;
use crate::scene::Scene;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct PreparedLogicalResourceStats {
    pub(in crate::render) materials: u64,
    pub(in crate::render) textures: u64,
    pub(in crate::render) environments: u64,
    pub(in crate::render) live_logical_handles: u64,
}

pub(in crate::render) fn collect_logical_resource_stats<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
    environment_count: u64,
) -> PreparedLogicalResourceStats {
    let mut geometries = HashSet::new();
    let mut materials = HashSet::new();
    let mut textures = HashSet::new();

    for (_node, mesh, _transform) in scene.mesh_nodes() {
        geometries.insert(mesh.geometry());
        materials.insert(mesh.material());
        collect_material_textures(assets, mesh.material(), &mut textures);
    }

    for (_node, instance_set, _transform) in scene.instance_set_nodes() {
        geometries.insert(instance_set.geometry());
        materials.insert(instance_set.material());
        collect_material_textures(assets, instance_set.material(), &mut textures);
    }

    let materials = materials.len() as u64;
    let textures = textures.len() as u64;
    let environments = environment_count;
    let live_logical_handles = geometries.len() as u64 + materials + textures + environments;

    PreparedLogicalResourceStats {
        materials,
        textures,
        environments,
        live_logical_handles,
    }
}

fn collect_material_textures<F>(
    assets: Option<&Assets<F>>,
    material: crate::assets::MaterialHandle,
    textures: &mut HashSet<crate::assets::TextureHandle>,
) {
    let Some(assets) = assets else {
        return;
    };
    let Some(material) = assets.material(material) else {
        return;
    };
    for texture in [
        material.base_color_texture(),
        material.normal_texture(),
        material.metallic_roughness_texture(),
        material.occlusion_texture(),
        material.emissive_texture(),
    ]
    .into_iter()
    .flatten()
    {
        if assets.texture(texture).is_some() {
            textures.insert(texture);
        }
    }
}
