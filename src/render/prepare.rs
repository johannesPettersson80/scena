use std::collections::HashSet;

use crate::assets::{Assets, EnvironmentDesc};
use crate::diagnostics::{
    Backend, Capabilities, CapabilityStatus, Diagnostic, DiagnosticCode, PrepareError,
};
use crate::geometry::{GeometryDesc, GeometryTopology, Primitive, Vertex};
use crate::material::{AlphaMode, Color, MaterialDesc, MaterialKind};
use crate::scene::{Camera, Light, NodeKey, Quat, Scene, Transform, Vec3};

use super::RasterTarget;

mod strokes;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct PreparedDepthStats {
    pub(super) passes: u64,
    pub(super) draws: u64,
    pub(super) reversed_z: bool,
}

pub(super) fn collect_prepared_primitives<F>(
    target: RasterTarget,
    scene: &Scene,
    assets: Option<&Assets<F>>,
) -> Result<Vec<Primitive>, PrepareError> {
    if let Some(model_node) = scene.model_nodes().next() {
        return Err(PrepareError::UnsupportedModelNode { node: model_node });
    }

    let origin_shift = scene.origin_shift();
    let mut primitives: Vec<Primitive> = scene
        .renderables()
        .flat_map(|(renderable, transform)| {
            renderable
                .primitives()
                .iter()
                .map(move |primitive| transform_primitive(primitive, transform, origin_shift))
        })
        .collect();
    let mut transparent_primitives = Vec::new();

    for (node, mesh, transform) in scene.mesh_nodes() {
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
            PrimitiveBakeParams {
                target,
                transform,
                origin_shift,
            },
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

pub(super) fn collect_lighting_stats(
    scene: &Scene,
    backend: Backend,
) -> Result<PreparedLightingStats, PrepareError> {
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
        let capabilities = Capabilities::for_backend(backend);
        PreparedLightingStats {
            shadow_maps: 1,
            directional_shadow_map_resolution: Some(
                capabilities.directional_shadow_map_default_size,
            ),
            directional_shadow_pcf_kernel: Some(DIRECTIONAL_SHADOW_PCF_KERNEL),
        }
    } else {
        PreparedLightingStats::default()
    })
}

pub(super) fn collect_precision_diagnostics(scene: &Scene, backend: Backend) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for (node, transform) in scene.node_transforms() {
        let relative_translation = subtract_vec3(transform.translation, scene.origin_shift());
        let absolute_magnitude = transform
            .translation
            .x
            .abs()
            .max(transform.translation.y.abs())
            .max(transform.translation.z.abs());
        let magnitude = relative_translation
            .x
            .abs()
            .max(relative_translation.y.abs())
            .max(relative_translation.z.abs());
        if absolute_magnitude >= LARGE_SCENE_TRANSLATION_WARNING
            && magnitude >= LARGE_SCENE_TRANSLATION_WARNING
        {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::LargeScenePrecisionRisk,
                format!(
                    "node {node:?} is {magnitude:.1} scene units from the origin; f32 transform precision may be visible"
                ),
                "use camera-relative rendering or an origin-shift policy for large-world scenes",
            ));
        }
    }

    for (node, _camera, camera) in scene.camera_nodes() {
        let (near, far) = match camera {
            Camera::Perspective(camera) => (camera.near, camera.far),
            Camera::Orthographic(camera) => (camera.near, camera.far),
        };
        if near > 0.0 && far.is_finite() && near.is_finite() {
            let ratio = far / near;
            if ratio > DEPTH_RANGE_RATIO_WARNING {
                diagnostics.push(Diagnostic::warning(
                    DiagnosticCode::DepthPrecisionRisk,
                    format!(
                        "camera node {node:?} has far/near ratio {ratio:.0}; depth precision may cause z-fighting"
                    ),
                    "use DepthRange::fit_sphere for focused views or tighten camera near/far planes",
                ));
            }
        }
    }

    if backend == Backend::WebGl2 {
        diagnostics.push(Diagnostic::warning(
            DiagnosticCode::WebGl2DepthCompatibility,
            "WebGL2 disables reversed-Z depth and uses the compatibility depth profile",
            "expect reduced far/near precision; tighten camera depth ranges for WebGL2 scenes",
        ));
    }

    diagnostics
}

pub(super) fn collect_depth_prepass_stats(
    primitives: &[Primitive],
    backend: Backend,
) -> PreparedDepthStats {
    if primitives.is_empty() {
        PreparedDepthStats::default()
    } else {
        let capabilities = Capabilities::for_backend(backend);
        PreparedDepthStats {
            passes: 1,
            draws: primitives.len() as u64,
            reversed_z: capabilities.reversed_z_depth == CapabilityStatus::Supported,
        }
    }
}

const LARGE_SCENE_TRANSLATION_WARNING: f32 = 10_000.0;
const DEPTH_RANGE_RATIO_WARNING: f32 = 100_000.0;

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

    for (_node, mesh, _transform) in scene.mesh_nodes() {
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
struct PrimitiveBakeParams {
    target: RasterTarget,
    transform: Transform,
    origin_shift: Vec3,
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
    params: PrimitiveBakeParams,
    primitives: &mut Vec<Primitive>,
    transparent_primitives: &mut Vec<TransparentPrimitive>,
) -> Result<(), PrepareError> {
    match geometry.topology() {
        GeometryTopology::Triangles => append_triangle_primitives(
            node,
            geometry,
            material,
            params,
            primitives,
            transparent_primitives,
        ),
        GeometryTopology::Lines => {
            strokes::append_line_primitives(node, geometry, material, params.target, primitives)
        }
    }
}

fn append_triangle_primitives(
    node: NodeKey,
    geometry: &GeometryDesc,
    material: &MaterialDesc,
    params: PrimitiveBakeParams,
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
                node,
                geometry,
                material,
                params.target,
                primitives,
            );
        }
        MaterialKind::Edge => {
            return strokes::append_edge_primitives(
                node,
                geometry,
                material,
                params.target,
                primitives,
            );
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
                position: transform_position(
                    vertices[triangle[0] as usize].position,
                    params.transform,
                    params.origin_shift,
                ),
                color,
            },
            Vertex {
                position: transform_position(
                    vertices[triangle[1] as usize].position,
                    params.transform,
                    params.origin_shift,
                ),
                color,
            },
            Vertex {
                position: transform_position(
                    vertices[triangle[2] as usize].position,
                    params.transform,
                    params.origin_shift,
                ),
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
    // M1/M2 depth sorting uses prepared scene-space z. View projection and camera-space
    // sorting remain separate dirty-state work.
    let vertices = primitive.vertices();
    (vertices[0].position.z + vertices[1].position.z + vertices[2].position.z) / 3.0
}

fn transform_primitive(
    primitive: &Primitive,
    transform: Transform,
    origin_shift: Vec3,
) -> Primitive {
    let [a, b, c] = primitive.vertices();
    Primitive::triangle([
        transform_vertex(*a, transform, origin_shift),
        transform_vertex(*b, transform, origin_shift),
        transform_vertex(*c, transform, origin_shift),
    ])
}

fn transform_vertex(vertex: Vertex, transform: Transform, origin_shift: Vec3) -> Vertex {
    Vertex {
        position: transform_position(vertex.position, transform, origin_shift),
        color: vertex.color,
    }
}

fn transform_position(position: Vec3, transform: Transform, origin_shift: Vec3) -> Vec3 {
    let scaled = Vec3::new(
        position.x * transform.scale.x,
        position.y * transform.scale.y,
        position.z * transform.scale.z,
    );
    let rotated = rotate_vec3(transform.rotation, scaled);
    subtract_vec3(add_vec3(rotated, transform.translation), origin_shift)
}

fn rotate_vec3(rotation: Quat, vector: Vec3) -> Vec3 {
    let length_squared = rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return vector;
    }
    let inverse_length = length_squared.sqrt().recip();
    let qx = rotation.x * inverse_length;
    let qy = rotation.y * inverse_length;
    let qz = rotation.z * inverse_length;
    let qw = rotation.w * inverse_length;
    let tx = 2.0 * (qy * vector.z - qz * vector.y);
    let ty = 2.0 * (qz * vector.x - qx * vector.z);
    let tz = 2.0 * (qx * vector.y - qy * vector.x);
    Vec3::new(
        vector.x + qw * tx + (qy * tz - qz * ty),
        vector.y + qw * ty + (qz * tx - qx * tz),
        vector.z + qw * tz + (qx * ty - qy * tx),
    )
}

fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}
