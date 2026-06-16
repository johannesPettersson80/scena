use std::collections::BTreeMap;

use super::{error_diagnostic, scene_host_error_diagnostic};
use crate::assets::DefaultAssetFetcher;
use crate::geometry::GeometryDesc;
use crate::material::MaterialDesc;
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeBuildResourceV1, SceneRecipeBuildTargetV1, SceneRecipeCameraV1,
    SceneRecipeColorV1, SceneRecipeDiagnosticV1, SceneRecipeGeometryV1, SceneRecipeLookAtTargetV1,
    SceneRecipeMaterialV1, SceneRecipeNodeV1, SceneRecipePrimitiveV1, SceneRecipeTransformV1,
};
use crate::scene_host::SceneHostCore;
use crate::scene_host::camera::controls_from_scene_camera;
use crate::{
    Color, GeometryHandle, MaterialHandle, NodeKey, PerspectiveCamera, Quat, Transform, Vec3,
};

pub(super) fn build_authored_geometries(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeGeometryV1],
    manifest: &mut Vec<SceneRecipeBuildResourceV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, GeometryHandle> {
    let mut handles = BTreeMap::new();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.geometries[{index}]");
        let (kind, geometry) = match authored_geometry(&recipe.primitive) {
            Ok(value) => value,
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path(format!("{path}.primitive")));
                continue;
            }
        };
        let vertex_count = geometry.vertices().len();
        let index_count = geometry.indices().len();
        if vertex_count > policy.max_vertices() {
            diagnostics.push(error_diagnostic(
                &path,
                "policy_violation",
                format!(
                    "geometry '{}' has {vertex_count} vertices, exceeding RecipeBuildPolicy max_vertices {}",
                    recipe.id,
                    policy.max_vertices()
                ),
                "simplify the geometry or raise the operator-owned max_vertices policy",
            ));
            continue;
        }
        if index_count > policy.max_indices() {
            diagnostics.push(error_diagnostic(
                &path,
                "policy_violation",
                format!(
                    "geometry '{}' has {index_count} indices, exceeding RecipeBuildPolicy max_indices {}",
                    recipe.id,
                    policy.max_indices()
                ),
                "simplify the geometry or raise the operator-owned max_indices policy",
            ));
            continue;
        }
        let handle = host.assets.create_geometry(geometry);
        handles.insert(recipe.id.clone(), handle);
        manifest.push(SceneRecipeBuildResourceV1 {
            id: recipe.id.clone(),
            kind,
            vertex_count: Some(vertex_count),
            index_count: Some(index_count),
        });
    }
    handles
}

fn authored_geometry(
    primitive: &SceneRecipePrimitiveV1,
) -> Result<(String, GeometryDesc), Box<SceneRecipeDiagnosticV1>> {
    match primitive.kind.as_str() {
        "box" => {
            let [width, height, depth] = primitive.size.ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_primitive",
                    "box primitive requires a finite positive size",
                    "emit primitive:{kind:\"box\",size:[width,height,depth]}",
                ))
            })?;
            Ok((
                "box".to_owned(),
                GeometryDesc::box_xyz(width as f32, height as f32, depth as f32),
            ))
        }
        kind => Err(Box::new(error_diagnostic(
            "$",
            "unsupported_feature",
            format!("primitive kind '{kind}' is not implemented in this slice"),
            "use kind:\"box\" until the primitive-coverage slice lands",
        ))),
    }
}

pub(super) fn build_authored_materials(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    recipes: &[SceneRecipeMaterialV1],
    manifest: &mut Vec<SceneRecipeBuildResourceV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, MaterialHandle> {
    let mut handles = BTreeMap::new();
    if recipes.len() > policy.max_materials() {
        diagnostics.push(error_diagnostic(
            "$.materials",
            "policy_violation",
            format!(
                "recipe declares {} authored materials, exceeding RecipeBuildPolicy max_materials {}",
                recipes.len(),
                policy.max_materials()
            ),
            "reduce material count or raise the operator-owned max_materials policy",
        ));
        return handles;
    }
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.materials[{index}]");
        let base_color = match authored_color(colors, &recipe.base_color) {
            Ok(color) => color,
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path(format!("{path}.base_color")));
                continue;
            }
        };
        let (kind, material) = match recipe.kind.as_str() {
            "unlit" => ("unlit", MaterialDesc::unlit(base_color)),
            "pbr_metallic_roughness" => (
                "pbr_metallic_roughness",
                MaterialDesc::pbr_metallic_roughness(
                    base_color,
                    recipe.metallic.unwrap_or(0.0) as f32,
                    recipe.roughness.unwrap_or(1.0) as f32,
                ),
            ),
            kind => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "unsupported_feature",
                    format!("material kind '{kind}' is not implemented in this slice"),
                    "use kind:\"unlit\" or kind:\"pbr_metallic_roughness\"",
                ));
                continue;
            }
        };
        let handle = host.assets.create_material(material);
        handles.insert(recipe.id.clone(), handle);
        manifest.push(SceneRecipeBuildResourceV1 {
            id: recipe.id.clone(),
            kind: kind.to_owned(),
            vertex_count: None,
            index_count: None,
        });
    }
    handles
}

fn authored_color(
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    value: &str,
) -> Result<Color, Box<SceneRecipeDiagnosticV1>> {
    if let Some(color) = colors.get(value) {
        return match color {
            SceneRecipeColorV1::Hex(hex) => Color::from_hex(hex).map_err(|error| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_color",
                    format!("color '{value}' is not valid hex: {error}"),
                    "use a six-digit #RRGGBB color",
                ))
            }),
        };
    }
    Color::from_hex(value).map_err(|error| {
        Box::new(error_diagnostic(
            "$",
            "unknown_color_ref",
            format!("base_color '{value}' is not a declared color or direct hex value: {error}"),
            "reference a key from colors or use a direct #RRGGBB value",
        ))
    })
}

pub(super) fn build_authored_nodes(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeNodeV1],
    geometries: &BTreeMap<String, GeometryHandle>,
    materials: &BTreeMap<String, MaterialHandle>,
    manifest: &mut Vec<SceneRecipeBuildTargetV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, NodeKey> {
    let mut node_keys = BTreeMap::new();
    if recipes.len() > policy.max_nodes() {
        diagnostics.push(error_diagnostic(
            "$.nodes",
            "policy_violation",
            format!(
                "recipe declares {} authored nodes, exceeding RecipeBuildPolicy max_nodes {}",
                recipes.len(),
                policy.max_nodes()
            ),
            "reduce node count or raise the operator-owned max_nodes policy",
        ));
        return node_keys;
    }
    let root = host.scene.root();
    let root_handle = host.root_handle();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.nodes[{index}]");
        let Some(geometry) = geometries.get(&recipe.geometry).copied() else {
            diagnostics.push(error_diagnostic(
                &path,
                "unknown_geometry_ref",
                format!(
                    "node '{}' references missing geometry '{}'",
                    recipe.id, recipe.geometry
                ),
                "declare the geometry before the node",
            ));
            continue;
        };
        let Some(material) = materials.get(&recipe.material).copied() else {
            diagnostics.push(error_diagnostic(
                &path,
                "unknown_material_ref",
                format!(
                    "node '{}' references missing material '{}'",
                    recipe.id, recipe.material
                ),
                "declare the material before the node",
            ));
            continue;
        };
        let transform =
            match transform_from_recipe(recipe.transform.as_ref(), &BTreeMap::new(), host) {
                Ok(transform) => transform,
                Err(diagnostic) => {
                    diagnostics.push((*diagnostic).with_path(format!("{path}.transform")));
                    continue;
                }
            };
        let node = match host
            .scene
            .mesh(geometry, material)
            .parent(root)
            .transform(transform)
            .add()
        {
            Ok(node) => node,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "node_create_failed",
                    format!("failed to create node '{}': {error}", recipe.id),
                    "check the node parent, geometry, and material references",
                ));
                continue;
            }
        };
        let handle = host.register_node(node);
        node_keys.insert(recipe.id.clone(), node);
        manifest.push(SceneRecipeBuildTargetV1 {
            id: recipe.id.clone(),
            handle,
            kind: "node".to_owned(),
            parent: Some(root_handle),
            name: recipe.name.clone(),
            active: None,
        });
    }
    node_keys
}

pub(super) fn build_authored_cameras(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeCameraV1],
    node_keys: &BTreeMap<String, NodeKey>,
    manifest: &mut Vec<SceneRecipeBuildTargetV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let root = host.scene.root();
    let root_handle = host.root_handle();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.cameras[{index}]");
        if recipe.kind != "perspective" {
            diagnostics.push(error_diagnostic(
                &path,
                "unsupported_feature",
                format!(
                    "camera kind '{}' is not implemented in this slice",
                    recipe.kind
                ),
                "use kind:\"perspective\"",
            ));
            continue;
        }
        let camera = PerspectiveCamera::default()
            .with_fov_degrees(recipe.fov_degrees.unwrap_or(60.0) as f32)
            .with_aspect(host.viewport.logical_width() / host.viewport.logical_height());
        let transform = match transform_from_recipe(recipe.transform.as_ref(), node_keys, host) {
            Ok(transform) => transform,
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path(format!("{path}.transform")));
                continue;
            }
        };
        let camera_key = match host.scene.add_perspective_camera(root, camera, transform) {
            Ok(camera) => camera,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "camera_create_failed",
                    format!("failed to create camera '{}': {error}", recipe.id),
                    "check the camera transform and kind",
                ));
                continue;
            }
        };
        let Some(camera_node) = host.scene.camera_node(camera_key) else {
            diagnostics.push(error_diagnostic(
                &path,
                "camera_create_failed",
                format!("camera '{}' did not create a scene node", recipe.id),
                "report this as a scena bug",
            ));
            continue;
        };
        let handle = host.register_node(camera_node);
        if recipe.active {
            if let Err(error) = host.scene.set_active_camera(camera_key) {
                diagnostics.push(error_diagnostic(
                    &path,
                    "camera_activate_failed",
                    format!("failed to activate camera '{}': {error}", recipe.id),
                    "check the camera handle",
                ));
            } else {
                host.active_camera = camera_key;
                match controls_from_scene_camera(&host.scene, host.active_camera, Vec3::ZERO) {
                    Ok(controls) => host.camera_controls = controls,
                    Err(error) => diagnostics.push(scene_host_error_diagnostic(
                        &path,
                        "camera_controls_failed",
                        error,
                    )),
                }
            }
        }
        manifest.push(SceneRecipeBuildTargetV1 {
            id: recipe.id.clone(),
            handle,
            kind: "camera".to_owned(),
            parent: Some(root_handle),
            name: None,
            active: Some(recipe.active),
        });
    }
}

fn transform_from_recipe(
    transform: Option<&SceneRecipeTransformV1>,
    node_keys: &BTreeMap<String, NodeKey>,
    host: &SceneHostCore<DefaultAssetFetcher>,
) -> Result<Transform, Box<SceneRecipeDiagnosticV1>> {
    let Some(transform) = transform else {
        return Ok(Transform::IDENTITY);
    };
    match transform {
        SceneRecipeTransformV1::Raw {
            translation,
            rotation,
            scale,
        } => {
            let rotation = Quat::from_xyzw(
                rotation[0] as f32,
                rotation[1] as f32,
                rotation[2] as f32,
                rotation[3] as f32,
            );
            let length_sq = rotation.length_squared();
            if !length_sq.is_finite() || length_sq <= f32::EPSILON {
                return Err(Box::new(error_diagnostic(
                    "$",
                    "invalid_rotation",
                    "raw transform rotation must be a finite non-zero quaternion",
                    "use [0,0,0,1] for identity",
                )));
            }
            Ok(Transform {
                translation: vec3(*translation),
                rotation: rotation.normalize(),
                scale: vec3(*scale),
            })
        }
        SceneRecipeTransformV1::Trs {
            translation,
            rotation_degrees,
            scale,
        } => Ok(Transform::IDENTITY
            .with_translation(vec3(*translation))
            .rotate_x_deg(rotation_degrees[0] as f32)
            .rotate_y_deg(rotation_degrees[1] as f32)
            .rotate_z_deg(rotation_degrees[2] as f32)
            .with_scale(vec3(*scale))),
        SceneRecipeTransformV1::LookAt { eye, target, up } => {
            let target = match target {
                SceneRecipeLookAtTargetV1::Position(position) => vec3(*position),
                SceneRecipeLookAtTargetV1::Node(id) => {
                    let node = node_keys.get(id).ok_or_else(|| {
                        Box::new(error_diagnostic(
                            "$",
                            "unknown_node_ref",
                            format!("look_at target references unknown node '{id}'"),
                            "target an authored node id or provide a [x,y,z] position",
                        ))
                    })?;
                    node_target_position(host, *node).ok_or_else(|| {
                        Box::new(error_diagnostic(
                            "$",
                            "node_bounds_missing",
                            format!("look_at target node '{id}' has no position or bounds"),
                            "target a renderable node or provide an explicit [x,y,z] position",
                        ))
                    })?
                }
            };
            Ok(Transform::at(vec3(*eye)).looking_at(target, vec3(*up)))
        }
    }
}

trait TransformScaleExt {
    fn with_scale(self, scale: Vec3) -> Self;
}

impl TransformScaleExt for Transform {
    fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }
}

fn node_target_position(host: &SceneHostCore<DefaultAssetFetcher>, node: NodeKey) -> Option<Vec3> {
    host.scene
        .node_world_bounds(node, &host.assets)
        .ok()
        .flatten()
        .map(|bounds| bounds.center())
        .or_else(|| {
            host.scene
                .world_transform(node)
                .map(|transform| transform.translation)
        })
}

fn vec3(value: [f64; 3]) -> Vec3 {
    Vec3::new(value[0] as f32, value[1] as f32, value[2] as f32)
}

trait DiagnosticPathExt {
    fn with_path(self, path: String) -> Self;
}

impl DiagnosticPathExt for SceneRecipeDiagnosticV1 {
    fn with_path(mut self, path: String) -> Self {
        self.path = path;
        self
    }
}
