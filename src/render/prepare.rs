use crate::assets::Assets;
use crate::diagnostics::PrepareError;
use crate::geometry::{GeometryDesc, GeometryTopology, Primitive, Vertex};
use crate::material::{AlphaMode, Color, MaterialDesc, MaterialKind};
use crate::scene::{NodeKey, Scene, Vec3};

use super::RasterTarget;

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
        GeometryTopology::Triangles => {
            append_triangle_primitives(node, geometry, material, primitives, transparent_primitives)
        }
        GeometryTopology::Lines => {
            append_line_primitives(node, geometry, material, target, primitives)
        }
    }
}

fn append_triangle_primitives(
    node: NodeKey,
    geometry: &GeometryDesc,
    material: &MaterialDesc,
    primitives: &mut Vec<Primitive>,
    transparent_primitives: &mut Vec<TransparentPrimitive>,
) -> Result<(), PrepareError> {
    if matches!(material.kind(), MaterialKind::Line) {
        return Err(PrepareError::UnsupportedMaterialKind {
            node,
            kind: material.kind(),
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

fn append_line_primitives(
    node: NodeKey,
    geometry: &GeometryDesc,
    material: &MaterialDesc,
    target: RasterTarget,
    primitives: &mut Vec<Primitive>,
) -> Result<(), PrepareError> {
    let (color, width_px) = line_material(node, material)?;
    let vertices = geometry.vertices();
    for segment in geometry.indices().chunks_exact(2) {
        append_line_segment(
            vertices[segment[0] as usize].position,
            vertices[segment[1] as usize].position,
            color,
            width_px,
            target,
            primitives,
        );
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

fn line_material(node: NodeKey, material: &MaterialDesc) -> Result<(Color, f32), PrepareError> {
    match material.kind() {
        MaterialKind::Line => {}
        MaterialKind::Unlit | MaterialKind::PbrMetallicRoughness => {
            return Err(PrepareError::UnsupportedGeometryTopology {
                node,
                topology: GeometryTopology::Lines,
            });
        }
        MaterialKind::Wireframe | MaterialKind::Edge => {
            return Err(PrepareError::UnsupportedMaterialKind {
                node,
                kind: material.kind(),
            });
        }
    }

    let mut color = material.base_color();
    match material.alpha_mode() {
        AlphaMode::Opaque => color.a = 1.0,
        AlphaMode::Mask { .. } | AlphaMode::Blend => {
            return Err(PrepareError::UnsupportedAlphaMode {
                node,
                alpha_mode: material.alpha_mode(),
            });
        }
    }
    Ok((color, material.stroke_width_px().unwrap_or(1.0)))
}

fn average_depth(primitive: &Primitive) -> f32 {
    // M1 foundation depth uses local-space z. Node transforms and view projection are not
    // applied until the scene transform/camera dirty-state work lands.
    let vertices = primitive.vertices();
    (vertices[0].position.z + vertices[1].position.z + vertices[2].position.z) / 3.0
}

fn append_line_segment(
    start: Vec3,
    end: Vec3,
    color: Color,
    width_px: f32,
    target: RasterTarget,
    primitives: &mut Vec<Primitive>,
) {
    let start = ScreenPoint::from_vec3(start, target);
    let end = ScreenPoint::from_vec3(end, target);
    let delta_x = end.x - start.x;
    let delta_y = end.y - start.y;
    let length = (delta_x * delta_x + delta_y * delta_y).sqrt();
    if length <= f32::EPSILON {
        return;
    }

    let half_width = width_px * 0.5;
    let normal_x = -delta_y / length * half_width;
    let normal_y = delta_x / length * half_width;
    let a = start.offset(normal_x, normal_y).to_vec3(target);
    let b = end.offset(normal_x, normal_y).to_vec3(target);
    let c = end.offset(-normal_x, -normal_y).to_vec3(target);
    let d = start.offset(-normal_x, -normal_y).to_vec3(target);

    primitives.push(Primitive::triangle([
        Vertex { position: a, color },
        Vertex { position: b, color },
        Vertex { position: c, color },
    ]));
    primitives.push(Primitive::triangle([
        Vertex { position: a, color },
        Vertex { position: c, color },
        Vertex { position: d, color },
    ]));
}

#[derive(Clone, Copy)]
struct ScreenPoint {
    x: f32,
    y: f32,
    z: f32,
}

impl ScreenPoint {
    fn from_vec3(position: Vec3, target: RasterTarget) -> Self {
        let width = target.width.saturating_sub(1) as f32;
        let height = target.height.saturating_sub(1) as f32;
        Self {
            x: (position.x * 0.5 + 0.5) * width,
            y: (1.0 - (position.y * 0.5 + 0.5)) * height,
            z: position.z,
        }
    }

    fn offset(self, x: f32, y: f32) -> Self {
        Self {
            x: self.x + x,
            y: self.y + y,
            z: self.z,
        }
    }

    fn to_vec3(self, target: RasterTarget) -> Vec3 {
        Vec3::new(
            screen_x_to_ndc(self.x, target),
            screen_y_to_ndc(self.y, target),
            self.z,
        )
    }
}

fn screen_x_to_ndc(x: f32, target: RasterTarget) -> f32 {
    if target.width <= 1 {
        0.0
    } else {
        (x / target.width.saturating_sub(1) as f32 - 0.5) * 2.0
    }
}

fn screen_y_to_ndc(y: f32, target: RasterTarget) -> f32 {
    if target.height <= 1 {
        0.0
    } else {
        ((1.0 - y / target.height.saturating_sub(1) as f32) - 0.5) * 2.0
    }
}
