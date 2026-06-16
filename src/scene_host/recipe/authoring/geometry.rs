use std::collections::BTreeMap;

use super::common::{
    DiagnosticPathExt, authored_color, positive_f64, primitive_size2, primitive_size3, vec3,
};
use crate::assets::DefaultAssetFetcher;
use crate::geometry::{GeometryDesc, GeometryTopology, GeometryVertex};
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeBuildResourceV1, SceneRecipeColorV1, SceneRecipeDiagnosticV1,
    SceneRecipeGeometryV1, SceneRecipeMeshV1,
};
use crate::scene_host::SceneHostCore;
use crate::{Color, GeometryHandle, Vec3};

use super::super::error_diagnostic;

pub(in crate::scene_host::recipe) fn build_authored_geometries(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    recipes: &[SceneRecipeGeometryV1],
    manifest: &mut Vec<SceneRecipeBuildResourceV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, GeometryHandle> {
    let mut handles = BTreeMap::new();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.geometries[{index}]");
        let (kind, geometry) = match authored_geometry(recipe, colors) {
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
    recipe: &SceneRecipeGeometryV1,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
) -> Result<(String, GeometryDesc), Box<SceneRecipeDiagnosticV1>> {
    if let Some(mesh) = &recipe.mesh {
        return authored_mesh(mesh, colors).map(|geometry| ("mesh".to_owned(), geometry));
    }
    let Some(primitive) = &recipe.primitive else {
        return Err(Box::new(error_diagnostic(
            "$",
            "missing_geometry_source",
            "geometry must include primitive or mesh",
            "emit either primitive:{...} or mesh:{...}",
        )));
    };
    match primitive.kind.as_str() {
        "box" => {
            let [width, height, depth] = primitive_size3(primitive).ok_or_else(|| {
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
        "plane" => {
            let [width, depth] = primitive_size2(primitive).ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_primitive",
                    "plane primitive requires finite positive size",
                    "emit primitive:{kind:\"plane\",size:[width,depth]}",
                ))
            })?;
            Ok((
                "plane".to_owned(),
                GeometryDesc::plane(width as f32, depth as f32),
            ))
        }
        "sphere" => {
            let radius = positive_f64(primitive.radius).ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_primitive",
                    "sphere primitive requires a finite positive radius",
                    "emit primitive:{kind:\"sphere\",radius}",
                ))
            })?;
            Ok((
                "sphere".to_owned(),
                GeometryDesc::sphere(
                    radius as f32,
                    primitive.segments.unwrap_or(32),
                    primitive.rings.unwrap_or(16),
                ),
            ))
        }
        "cylinder" => {
            let radius = positive_f64(primitive.radius).ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_primitive",
                    "cylinder primitive requires a finite positive radius",
                    "emit primitive:{kind:\"cylinder\",radius,height}",
                ))
            })?;
            let height = positive_f64(primitive.height).ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_primitive",
                    "cylinder primitive requires a finite positive height",
                    "emit primitive:{kind:\"cylinder\",radius,height}",
                ))
            })?;
            Ok((
                "cylinder".to_owned(),
                GeometryDesc::cylinder(
                    radius as f32,
                    height as f32,
                    primitive.segments.unwrap_or(32),
                ),
            ))
        }
        "line" => {
            let start = primitive.start.ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_primitive",
                    "line primitive requires start",
                    "emit start:[x,y,z]",
                ))
            })?;
            let end = primitive.end.ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_primitive",
                    "line primitive requires end",
                    "emit end:[x,y,z]",
                ))
            })?;
            Ok((
                "line".to_owned(),
                GeometryDesc::line(vec3(start), vec3(end)),
            ))
        }
        "polyline" => Ok((
            "polyline".to_owned(),
            GeometryDesc::polyline(
                &primitive
                    .points
                    .iter()
                    .copied()
                    .map(vec3)
                    .collect::<Vec<_>>(),
            ),
        )),
        "arrow" => {
            let start = primitive.start.ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_primitive",
                    "arrow primitive requires start",
                    "emit start:[x,y,z]",
                ))
            })?;
            let end = primitive.end.ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_primitive",
                    "arrow primitive requires end",
                    "emit end:[x,y,z]",
                ))
            })?;
            Ok((
                "arrow".to_owned(),
                GeometryDesc::arrow(vec3(start), vec3(end)),
            ))
        }
        "grid" => {
            let size = positive_f64(
                primitive
                    .size
                    .as_ref()
                    .and_then(|size| size.first().copied()),
            )
            .or_else(|| positive_f64(primitive.length))
            .ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_primitive",
                    "grid primitive requires a finite positive size",
                    "emit primitive:{kind:\"grid\",size:[size],divisions?}",
                ))
            })?;
            Ok((
                "grid".to_owned(),
                GeometryDesc::grid(size as f32, primitive.divisions.unwrap_or(10)),
            ))
        }
        "axes" => {
            let length = positive_f64(primitive.length).ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_primitive",
                    "axes primitive requires a finite positive length",
                    "emit primitive:{kind:\"axes\",length}",
                ))
            })?;
            Ok(("axes".to_owned(), GeometryDesc::axes(length as f32)))
        }
        kind => Err(Box::new(error_diagnostic(
            "$",
            "unsupported_feature",
            format!("primitive kind '{kind}' is not implemented in this slice"),
            "use kind:\"box\" until the primitive-coverage slice lands",
        ))),
    }
}

fn authored_mesh(
    mesh: &SceneRecipeMeshV1,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
) -> Result<GeometryDesc, Box<SceneRecipeDiagnosticV1>> {
    let topology = match mesh.topology.as_str() {
        "triangles" => GeometryTopology::Triangles,
        "lines" => GeometryTopology::Lines,
        _ => {
            return Err(Box::new(error_diagnostic(
                "$",
                "unsupported_feature",
                format!("mesh topology '{}' is not supported", mesh.topology),
                "use topology:\"triangles\" or topology:\"lines\"",
            )));
        }
    };
    let vertices = mesh
        .positions
        .iter()
        .enumerate()
        .map(|(index, position)| GeometryVertex {
            position: vec3(*position),
            normal: mesh
                .normals
                .get(index)
                .copied()
                .map(vec3)
                .unwrap_or(Vec3::new(0.0, 1.0, 0.0)),
        })
        .collect::<Vec<_>>();
    let colors = if mesh.colors.is_empty() {
        vec![Color::WHITE; vertices.len()]
    } else {
        mesh.colors
            .iter()
            .map(|color| authored_color(colors, color).map_err(|diagnostic| *diagnostic))
            .collect::<Result<Vec<_>, _>>()
            .map_err(Box::new)?
    };
    let uvs = if mesh.uvs.is_empty() {
        vec![[0.0, 0.0]; vertices.len()]
    } else {
        mesh.uvs
            .iter()
            .map(|uv| [uv[0] as f32, uv[1] as f32])
            .collect::<Vec<_>>()
    };
    GeometryDesc::try_new_with_vertex_colors_and_tex_coords(
        topology,
        vertices,
        mesh.indices.clone(),
        colors,
        uvs,
    )
    .map_err(|error| {
        Box::new(error_diagnostic(
            "$",
            "invalid_mesh",
            format!("custom mesh is invalid: {error:?}"),
            "fix positions, normals, colors, uvs, and indices",
        ))
    })
}
