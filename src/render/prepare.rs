use crate::assets::Assets;
use crate::diagnostics::PrepareError;
use crate::geometry::{GeometryDesc, GeometryTopology, Primitive, Vertex};
use crate::material::{AlphaMode, Color, MaterialDesc, MaterialKind};
use crate::scene::{NodeKey, Scene};

pub(super) fn collect_prepared_primitives<F>(
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
        append_geometry_primitives(node, &geometry, &material, &mut primitives)?;
    }

    Ok(primitives)
}

fn append_geometry_primitives(
    node: NodeKey,
    geometry: &GeometryDesc,
    material: &MaterialDesc,
    primitives: &mut Vec<Primitive>,
) -> Result<(), PrepareError> {
    if geometry.topology() != GeometryTopology::Triangles {
        return Err(PrepareError::UnsupportedGeometryTopology {
            node,
            topology: geometry.topology(),
        });
    }

    let color = forward_opaque_color(node, material)?;

    for triangle in geometry.indices().chunks_exact(3) {
        let vertices = geometry.vertices();
        primitives.push(Primitive::triangle([
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
        ]));
    }

    Ok(())
}

fn forward_opaque_color(node: NodeKey, material: &MaterialDesc) -> Result<Color, PrepareError> {
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
            Ok(color)
        }
        AlphaMode::Mask { .. } | AlphaMode::Blend => Err(PrepareError::UnsupportedAlphaMode {
            node,
            alpha_mode: material.alpha_mode(),
        }),
    }
}
