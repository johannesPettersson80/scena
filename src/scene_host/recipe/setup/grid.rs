use std::collections::BTreeMap;

use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    SceneRecipeColorV1, SceneRecipeDiagnosticV1, SceneRecipeGridReflectionV1, SceneRecipeGridV1,
};
use crate::scene_host::SceneHostCore;
use crate::{Aabb, Color, GeometryDesc, GridFloorOptions, MaterialDesc, Transform, Vec3};

use super::super::authoring::{DiagnosticPathExt, authored_color};
use super::super::scene_host_error_diagnostic;

pub(super) fn apply_grid(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    grid: &SceneRecipeGridV1,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let mut options = GridFloorOptions::new();
    if grid.under_bounds {
        options = grid_options_under_scene_bounds(host, options);
    }
    if let Some(floor_y) = grid.floor_y {
        options = options.floor_y(floor_y as f32);
    }
    if let Some(padding) = grid.padding {
        options = options.padding(padding as f32);
    }
    if let Some(line_spacing) = grid.line_spacing {
        options = options.line_spacing(line_spacing as f32);
    }
    if let Some(line_width_px) = grid.line_width_px {
        options = options.line_width_px(line_width_px as f32);
    }
    if let Some(color) = grid.color.as_deref() {
        match authored_color(colors, color) {
            Ok(color) => options = options.color(color),
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path("$.scene.grid.color".to_owned()));
                return;
            }
        }
    }
    if let Some(color) = grid.line_color.as_deref() {
        match authored_color(colors, color) {
            Ok(color) => options = options.line_color(color),
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path("$.scene.grid.line_color".to_owned()));
                return;
            }
        }
    }
    if let Some(roughness) = grid.roughness {
        options = options.roughness(roughness as f32);
    }
    let reflection_sources = grid
        .reflection
        .filter(|reflection| reflection.enabled)
        .map(|reflection| collect_reflection_sources(host, options.floor_y_value(), reflection))
        .unwrap_or_default();
    add_grid_floor_with_sources(
        host,
        options,
        grid.reflection,
        &reflection_sources,
        diagnostics,
    );
}

pub(super) fn grid_options_under_scene_bounds(
    host: &SceneHostCore<DefaultAssetFetcher>,
    mut options: GridFloorOptions,
) -> GridFloorOptions {
    if let Ok(Some(bounds)) = host
        .scene
        .node_world_bounds(host.scene.root(), &host.assets)
    {
        options = options.under_bounds(bounds);
    }
    options
}

pub(super) fn add_grid_floor_with_options(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    options: GridFloorOptions,
    reflection: Option<SceneRecipeGridReflectionV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let reflection_sources = reflection
        .filter(|reflection| reflection.enabled)
        .map(|reflection| collect_reflection_sources(host, options.floor_y_value(), reflection))
        .unwrap_or_default();
    add_grid_floor_with_sources(host, options, reflection, &reflection_sources, diagnostics);
}

fn add_grid_floor_with_sources(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    options: GridFloorOptions,
    reflection: Option<SceneRecipeGridReflectionV1>,
    reflection_sources: &[GridReflectionSource],
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    match host.scene.add_grid_floor(&host.assets, options) {
        Ok(handles) => {
            host.register_node(handles.slab);
            host.register_node(handles.grid);
            add_grid_reflection_decals(
                host,
                handles.bounds.min.y,
                reflection_sources,
                reflection,
                diagnostics,
            );
        }
        Err(error) => diagnostics.push(scene_host_error_diagnostic(
            "$.scene.grid",
            "grid_create_failed",
            error.into(),
        )),
    }
}

#[derive(Debug, Clone, Copy)]
struct GridReflectionSource {
    center: Vec3,
    width: f32,
    depth: f32,
    height: f32,
    color: Color,
}

fn collect_reflection_sources(
    host: &SceneHostCore<DefaultAssetFetcher>,
    floor_y: f32,
    reflection: SceneRecipeGridReflectionV1,
) -> Vec<GridReflectionSource> {
    let strength = reflection.strength.unwrap_or(0.42) as f32;
    host.scene
        .mesh_nodes()
        .filter_map(|(node, mesh, _transform)| {
            let bounds = host
                .scene
                .node_world_bounds(node, &host.assets)
                .ok()
                .flatten()?;
            let extent = bounds.max - bounds.min;
            if extent.y <= 0.02 || bounds.max.y <= floor_y + 0.01 {
                return None;
            }
            let material = host.assets.material(mesh.material())?;
            let color = reflection_color(material.base_color(), strength);
            let group = top_level_ancestor(host, node);
            Some((group, bounds, color))
        })
        .fold(
            Vec::<(crate::scene::NodeKey, Aabb, Color, u32)>::new(),
            |mut groups, (group, bounds, color)| {
                // One decal per top-level object, not per mesh node. An
                // imported assembly can carry dozens of mesh nodes; emitting a
                // hard-edged quad for each produced a field of overlapping
                // rectangles instead of anything resembling a reflection.
                match groups.iter_mut().find(|(key, ..)| *key == group) {
                    Some((_, merged, merged_color, count)) => {
                        *merged = merged.union(bounds);
                        *merged_color = average_color(*merged_color, color, *count);
                        *count += 1;
                    }
                    None => groups.push((group, bounds, color, 1)),
                }
                groups
            },
        )
        .into_iter()
        .map(|(_, bounds, color, _)| {
            let extent = bounds.max - bounds.min;
            GridReflectionSource {
                center: bounds.center(),
                width: extent.x.max(0.08),
                depth: (extent.z + extent.y * 0.70).max(0.10),
                height: extent.y,
                color,
            }
        })
        .collect()
}

/// Returns the highest ancestor of `node` below the scene root, so every mesh
/// belonging to one imported object shares a single reflection group.
fn top_level_ancestor(
    host: &SceneHostCore<DefaultAssetFetcher>,
    node: crate::scene::NodeKey,
) -> crate::scene::NodeKey {
    let root = host.scene.root();
    let mut current = node;
    while let Some(parent) = host.scene.node(current).and_then(|node| node.parent()) {
        if parent == root {
            break;
        }
        current = parent;
    }
    current
}

fn average_color(accumulated: Color, next: Color, count: u32) -> Color {
    let weight = 1.0 / (count as f32 + 1.0);
    Color::from_linear_rgba(
        accumulated.r + (next.r - accumulated.r) * weight,
        accumulated.g + (next.g - accumulated.g) * weight,
        accumulated.b + (next.b - accumulated.b) * weight,
        accumulated.a,
    )
}

fn add_grid_reflection_decals(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    floor_y: f32,
    sources: &[GridReflectionSource],
    reflection: Option<SceneRecipeGridReflectionV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(reflection) = reflection.filter(|reflection| reflection.enabled) else {
        return;
    };
    let strength = reflection.strength.unwrap_or(0.42) as f32;
    for (index, source) in sources.iter().enumerate() {
        let geometry = host.assets.create_geometry(GeometryDesc::plane(
            source.width * (1.0 + strength * 0.20),
            source.depth,
        ));
        let material = host
            .assets
            .create_material(MaterialDesc::unlit(source.color).with_double_sided(true));
        let z_offset = (source.height * 0.10).min(source.depth * 0.25);
        let transform = Transform::at(Vec3::new(
            source.center.x,
            floor_y + 0.003 + index as f32 * 0.0005,
            source.center.z + z_offset,
        ));
        match host
            .scene
            .mesh(geometry, material)
            .transform(transform)
            .add()
        {
            Ok(node) => {
                host.register_node(node);
            }
            Err(error) => diagnostics.push(scene_host_error_diagnostic(
                "$.scene.grid.reflection",
                "grid_reflection_create_failed",
                error.into(),
            )),
        }
    }
}

fn reflection_color(color: Color, strength: f32) -> Color {
    let strength = strength.clamp(0.0, 1.0);
    Color::from_linear_rgba(
        (color.r * strength).clamp(0.0, 1.0),
        (color.g * strength).clamp(0.0, 1.0),
        (color.b * strength).clamp(0.0, 1.0),
        1.0,
    )
}
