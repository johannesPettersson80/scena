use std::collections::HashSet;

use crate::assets::{Assets, EnvironmentDesc};
use crate::diagnostics::PrepareError;
use crate::geometry::{GeometryDesc, GeometryTopology, Primitive, Vertex};
use crate::material::{AlphaMode, Color, MaterialDesc, MaterialKind};
use crate::scene::{Light, NodeKey, Scene};

use super::RasterTarget;

mod strokes;

pub(super) const DIRECTIONAL_SHADOW_MAP_RESOLUTION: u32 = 2048;
pub(super) const DIRECTIONAL_SHADOW_PCF_KERNEL: u8 = 3;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct PreparedLightingStats {
    pub(super) shadow_maps: u64,
    pub(super) directional_shadow_map_resolution: Option<u32>,
    pub(super) directional_shadow_pcf_kernel: Option<u8>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct PreparedEnvironmentStats {
    pub(super) cubemaps: u64,
    pub(super) prefilter_passes: u64,
    pub(super) brdf_luts: u64,
}

pub(super) fn collect_prepared_primitives<F>(
    target: RasterTarget,
    scene: &Scene,
    assets: Option<&Assets<F>>,
) -> Result<Vec<Primitive>, PrepareError> {
    if let Some(model_node) = scene.model_nodes().next() {
        return Err(PrepareError::UnsupportedModelNode { node: model_node });
    }

    let mut primitives: Vec<Primitive> = scene
        .renderables()
        .flat_map(|renderable| renderable.primitives().iter().cloned())
        .collect();
    let mut transparent_primitives = Vec::new();

    for (node, mesh) in scene.mesh_nodes() {
        let Some(assets) = assets else {
            return Err(PrepareError::AssetsRequired { node });
        };
        let geometry = assets
            .geometry(mesh.geometry())
            .ok_or(PrepareError::GeometryNotFound {
                node,
                geometry: mesh.geometry(),
            })?;
        let material = assets
            .material(mesh.material())
            .ok_or(PrepareError::MaterialNotFound {
                node,
                material: mesh.material(),
            })?;
        append_geometry_primitives(
            node,
            &geometry,
            &material,
            target,
            &mut primitives,
            &mut transparent_primitives,
        )?;
    }

    // Descending depth: larger local-space z is treated as farther for the M1 foundation.
    transparent_primitives
        .sort_by(|left: &TransparentPrimitive, right| right.depth.total_cmp(&left.depth));
    primitives.extend(
        transparent_primitives
            .into_iter()
            .map(|transparent| transparent.primitive),
    );

    Ok(primitives)
}

pub(super) fn collect_lighting_stats(scene: &Scene) -> Result<PreparedLightingStats, PrepareError> {
    let mut first_shadowed_directional = None;
    for (node, _light_key, light) in scene.light_nodes() {
        let Light::Directional(light) = light else {
            continue;
        };
        if !light.casts_shadows() {
            continue;
        }
        if let Some(first) = first_shadowed_directional {
            return Err(PrepareError::MultipleShadowedDirectionalLights {
                first,
                second: node,
            });
        }
        first_shadowed_directional = Some(node);
    }
    Ok(if first_shadowed_directional.is_some() {
        PreparedLightingStats {
            shadow_maps: 1,
            directional_shadow_map_resolution: Some(DIRECTIONAL_SHADOW_MAP_RESOLUTION),
            directional_shadow_pcf_kernel: Some(DIRECTIONAL_SHADOW_PCF_KERNEL),
        }
    } else {
        PreparedLightingStats::default()
    })
}

pub(super) fn collect_environment_prepare_stats(
    environment: Option<&EnvironmentDesc>,
) -> PreparedEnvironmentStats {
    match environment {
        Some(environment) if environment.is_equirectangular_hdr() => PreparedEnvironmentStats {
            cubemaps: 1,
            prefilter_passes: 1,
            brdf_luts: 1,
        },
        Some(_) | None => PreparedEnvironmentStats::default(),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct PreparedLogicalResourceStats {
    pub(super) materials: u64,
    pub(super) textures: u64,
    pub(super) environments: u64,
    pub(super) live_logical_handles: u64,
}

pub(super) fn collect_logical_resource_stats<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
    environment_count: u64,
) -> PreparedLogicalResourceStats {
    let mut geometries = HashSet::new();
    let mut materials = HashSet::new();
    let mut textures = HashSet::new();

    for (_node, mesh) in scene.mesh_nodes() {
        geometries.insert(mesh.geometry());
        materials.insert(mesh.material());

        let Some(assets) = assets else {
            continue;
        };
        let Some(material) = assets.material(mesh.material()) else {
            continue;
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

struct TransparentPrimitive {
    depth: f32,
    primitive: Primitive,
}

#[derive(Clone, Copy)]
enum MaterialPass {
    Opaque(Color),
    Blend(Color),
}

fn append_geometry_primitives(
    node: NodeKey,
    geometry: &GeometryDesc,
    material: &MaterialDesc,
    target: RasterTarget,
    primitives: &mut Vec<Primitive>,
    transparent_primitives: &mut Vec<TransparentPrimitive>,
) -> Result<(), PrepareError> {
    match geometry.topology() {
        GeometryTopology::Triangles => append_triangle_primitives(
            node,
            geometry,
            material,
            target,
            primitives,
            transparent_primitives,
        ),
        GeometryTopology::Lines => {
            strokes::append_line_primitives(node, geometry, material, target, primitives)
        }
    }
}

fn append_triangle_primitives(
    node: NodeKey,
    geometry: &GeometryDesc,
    material: &MaterialDesc,
    target: RasterTarget,
    primitives: &mut Vec<Primitive>,
    transparent_primitives: &mut Vec<TransparentPrimitive>,
) -> Result<(), PrepareError> {
    match material.kind() {
        MaterialKind::Unlit | MaterialKind::PbrMetallicRoughness => {}
        MaterialKind::Line => {
            return Err(PrepareError::UnsupportedMaterialKind {
                node,
                kind: material.kind(),
            });
        }
        MaterialKind::Wireframe => {
            return strokes::append_wireframe_primitives(
                node, geometry, material, target, primitives,
            );
        }
        MaterialKind::Edge => {
            return strokes::append_edge_primitives(node, geometry, material, target, primitives);
        }
    }

    let material_pass = material_pass(node, material)?;

    for triangle in geometry.indices().chunks_exact(3) {
        let vertices = geometry.vertices();
        let color = match material_pass {
            MaterialPass::Opaque(color) | MaterialPass::Blend(color) => color,
        };
        let primitive = Primitive::triangle([
            Vertex {
                position: vertices[triangle[0] as usize].position,
                color,
            },
            Vertex {
                position: vertices[triangle[1] as usize].position,
                color,
            },
            Vertex {
                position: vertices[triangle[2] as usize].position,
                color,
            },
        ]);
        match material_pass {
            MaterialPass::Opaque(_) => primitives.push(primitive),
            MaterialPass::Blend(_) => transparent_primitives.push(TransparentPrimitive {
                depth: average_depth(&primitive),
                primitive,
            }),
        }
    }

    Ok(())
}

fn material_pass(node: NodeKey, material: &MaterialDesc) -> Result<MaterialPass, PrepareError> {
    match material.kind() {
        MaterialKind::Unlit | MaterialKind::PbrMetallicRoughness => {}
        MaterialKind::Line | MaterialKind::Wireframe | MaterialKind::Edge => {
            return Err(PrepareError::UnsupportedMaterialKind {
                node,
                kind: material.kind(),
            });
        }
    }

    let mut color = material.base_color();
    let emissive = material.emissive();
    let emissive_strength = material.emissive_strength();
    color.r += emissive.r * emissive_strength;
    color.g += emissive.g * emissive_strength;
    color.b += emissive.b * emissive_strength;

    match material.alpha_mode() {
        AlphaMode::Opaque => {
            color.a = 1.0;
            Ok(MaterialPass::Opaque(color))
        }
        AlphaMode::Blend => Ok(MaterialPass::Blend(color)),
        AlphaMode::Mask { .. } => Err(PrepareError::UnsupportedAlphaMode {
            node,
            alpha_mode: material.alpha_mode(),
        }),
    }
}

fn average_depth(primitive: &Primitive) -> f32 {
    // M1 foundation depth uses local-space z. Node transforms and view projection are not
    // applied until the scene transform/camera dirty-state work lands.
    let vertices = primitive.vertices();
    (vertices[0].position.z + vertices[1].position.z + vertices[2].position.z) / 3.0
}
