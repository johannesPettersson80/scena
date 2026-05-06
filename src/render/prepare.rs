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
    primitives: &mut Vec<Primitive>,
    transparent_primitives: &mut Vec<TransparentPrimitive>,
) -> Result<(), PrepareError> {
    if geometry.topology() != GeometryTopology::Triangles {
        return Err(PrepareError::UnsupportedGeometryTopology {
            node,
            topology: geometry.topology(),
        });
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
