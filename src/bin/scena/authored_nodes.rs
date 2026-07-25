use super::*;

pub(super) fn run_authored_node_placement(
    args: &PlaceCommandArgs,
    source_text: &str,
    recipe: scena::SceneRecipeV1,
    verb: String,
) -> Result<CliOutcome, CliFailure> {
    let node_id = args.target_subject.id();
    let target = scena::ScenePlacementTargetV1::node(node_id);
    if matches!(verb.as_str(), "align_to_anchor" | "place_on") {
        let result = scena::ScenePlacementResultV1::failure_for_target(
            target,
            verb,
            scena::ScenePlacementDiagnosticV1::new(
                "import_only_verb",
                "$.verb",
                "anchor and connector placement requires an imported asset",
                "use --import for align_to_anchor/place_on, or use a bounds verb for --node",
            ),
        );
        return json_outcome(&result, 1, "failed to serialize placement result");
    }
    let Some(node_index) = recipe.nodes.iter().position(|node| node.id == node_id) else {
        let diagnostic = unknown_node_diagnostic(&recipe, node_id);
        let result = scena::ScenePlacementResultV1::failure_for_target(target, verb, diagnostic);
        return json_outcome(&result, 1, "failed to serialize placement result");
    };
    let current_world = match recipe_node_world_transform(&recipe, node_index) {
        Ok(transform) => transform,
        Err(diagnostic) => {
            let result =
                scena::ScenePlacementResultV1::failure_for_target(target, verb, *diagnostic);
            return json_outcome(&result, 1, "failed to serialize placement result");
        }
    };
    let desired_world = match verb.as_str() {
        "center" | "ground" | "fit_to_size" => {
            let bounds = match recipe_node_local_bounds(&recipe, node_index) {
                Ok(bounds) => bounds,
                Err(diagnostic) => {
                    let result = scena::ScenePlacementResultV1::failure_for_target(
                        target,
                        verb,
                        *diagnostic,
                    );
                    return json_outcome(&result, 1, "failed to serialize placement result");
                }
            };
            match verb.as_str() {
                "center" => Ok(scena::placement_center_transform(
                    bounds,
                    current_world,
                    args.target.unwrap_or(scena::Vec3::ZERO),
                )),
                "ground" => Ok(scena::placement_ground_transform(
                    bounds,
                    current_world,
                    args.ground_y.unwrap_or(0.0),
                )),
                _ => scena::placement_fit_to_size_transform(
                    bounds,
                    current_world,
                    args.min_size,
                    args.max_size,
                ),
            }
        }
        "look_at" => authored_node_look_at_target(args, &recipe, &verb).and_then(|point| {
            scena::placement_look_at_transform(
                current_world,
                point,
                args.up.unwrap_or(scena::Vec3::Y),
            )
        }),
        _ => Err(Box::new(
            scena::ScenePlacementDiagnosticV1::new(
                "unknown_verb",
                "$.verb",
                format!("placement verb '{}' is not supported", args.verb),
                "use center, ground, fit_to_size, or look_at for authored nodes",
            )
            .with_suggestion("center"),
        )),
    };
    let desired_world = match desired_world {
        Ok(transform) => transform,
        Err(diagnostic) => {
            let result =
                scena::ScenePlacementResultV1::failure_for_target(target, verb, *diagnostic);
            return json_outcome(&result, 1, "failed to serialize placement result");
        }
    };
    let local_transform = match recipe_node_local_from_world(&recipe, node_index, desired_world) {
        Ok(transform) => transform,
        Err(diagnostic) => {
            let result =
                scena::ScenePlacementResultV1::failure_for_target(target, verb, *diagnostic);
            return json_outcome(&result, 1, "failed to serialize placement result");
        }
    };
    let result = scena::ScenePlacementResultV1::success_for_target(target, verb, local_transform);
    if args.apply {
        return emit_recipe_patch(args, source_text, recipe, result);
    }
    json_outcome(&result, 0, "failed to serialize placement result")
}

pub(super) fn public_placement_target(target: &PlaceTargetArg) -> scena::ScenePlacementTargetV1 {
    match target {
        PlaceTargetArg::Import(id) => scena::ScenePlacementTargetV1::import(id),
        PlaceTargetArg::Node(id) => scena::ScenePlacementTargetV1::node(id),
    }
}

fn unknown_node_diagnostic(
    recipe: &scena::SceneRecipeV1,
    node_id: &str,
) -> scena::ScenePlacementDiagnosticV1 {
    if recipe.imports.iter().any(|import| import.id == node_id) {
        return scena::ScenePlacementDiagnosticV1::new(
            "wrong_target_namespace",
            "$.nodes",
            format!("'{node_id}' is an import, not an authored node"),
            format!("use --import {node_id}"),
        )
        .with_candidates(vec![format!("--import {node_id}")]);
    }
    let candidates = scena::nearest_name_candidates(
        node_id,
        recipe.nodes.iter().map(|node| node.id.as_str()),
        3,
    );
    let mut diagnostic = scena::ScenePlacementDiagnosticV1::new(
        "unknown_node",
        "$.nodes",
        format!("recipe node '{node_id}' was not found"),
        "pass a node id declared in the recipe",
    )
    .with_candidates(candidates.clone());
    if let Some(first) = candidates.first() {
        diagnostic = diagnostic.with_suggestion(first.clone());
    }
    diagnostic
}

fn recipe_node_local_transform(
    recipe: &scena::SceneRecipeV1,
    node_index: usize,
) -> Result<scena::Transform, Box<scena::ScenePlacementDiagnosticV1>> {
    recipe.nodes[node_index]
        .transform
        .as_ref()
        .map(scena::Transform::try_from)
        .transpose()
        .map(|transform| transform.unwrap_or(scena::Transform::IDENTITY))
        .map_err(|error| {
            Box::new(scena::ScenePlacementDiagnosticV1::new(
                "invalid_transform",
                format!("$.nodes[{node_index}].transform"),
                error.to_string(),
                "use a canonical kind:raw or kind:trs transform before placement",
            ))
        })
}

fn recipe_node_world_transform(
    recipe: &scena::SceneRecipeV1,
    node_index: usize,
) -> Result<scena::Transform, Box<scena::ScenePlacementDiagnosticV1>> {
    let local = recipe_node_local_transform(recipe, node_index)?;
    let Some(parent_id) = recipe.nodes[node_index].parent.as_deref() else {
        return Ok(local);
    };
    let Some(parent_index) = recipe.nodes.iter().position(|node| node.id == parent_id) else {
        return Err(Box::new(scena::ScenePlacementDiagnosticV1::new(
            "unknown_parent",
            format!("$.nodes[{node_index}].parent"),
            format!("parent node '{parent_id}' was not found"),
            "validate the recipe before placement",
        )));
    };
    Ok(scena::Transform::compose(
        recipe_node_world_transform(recipe, parent_index)?,
        local,
    ))
}

fn recipe_node_local_from_world(
    recipe: &scena::SceneRecipeV1,
    node_index: usize,
    world: scena::Transform,
) -> Result<scena::Transform, Box<scena::ScenePlacementDiagnosticV1>> {
    let Some(parent_id) = recipe.nodes[node_index].parent.as_deref() else {
        return Ok(world);
    };
    let parent_index = recipe
        .nodes
        .iter()
        .position(|node| node.id == parent_id)
        .ok_or_else(|| {
            Box::new(scena::ScenePlacementDiagnosticV1::new(
                "unknown_parent",
                format!("$.nodes[{node_index}].parent"),
                format!("parent node '{parent_id}' was not found"),
                "validate the recipe before placement",
            ))
        })?;
    let parent = recipe_node_world_transform(recipe, parent_index)?;
    if !parent.scale.is_finite() || parent.scale.abs().min_element() <= f32::EPSILON {
        return Err(Box::new(scena::ScenePlacementDiagnosticV1::new(
            "non_invertible_parent_transform",
            format!("$.nodes[{parent_index}].transform.scale"),
            "parent scale must be finite and non-zero to place a child node",
            "use a finite non-zero parent scale",
        )));
    }
    let inverse_rotation = parent.rotation.normalize().inverse();
    Ok(scena::Transform {
        translation: (inverse_rotation * (world.translation - parent.translation)) / parent.scale,
        rotation: (inverse_rotation * world.rotation).normalize(),
        scale: world.scale / parent.scale,
    })
}

fn recipe_node_local_bounds(
    recipe: &scena::SceneRecipeV1,
    node_index: usize,
) -> Result<scena::Aabb, Box<scena::ScenePlacementDiagnosticV1>> {
    let node = &recipe.nodes[node_index];
    let geometry_id = node.geometry.as_deref().ok_or_else(|| {
        Box::new(scena::ScenePlacementDiagnosticV1::new(
            "missing_bounds",
            format!("$.nodes[{node_index}].geometry"),
            "authored-node placement requires direct geometry bounds",
            "target a node with geometry or use an imported asset",
        ))
    })?;
    let geometry_index = recipe
        .geometries
        .iter()
        .position(|geometry| geometry.id == geometry_id)
        .ok_or_else(|| {
            Box::new(scena::ScenePlacementDiagnosticV1::new(
                "missing_bounds",
                format!("$.nodes[{node_index}].geometry"),
                format!("geometry '{geometry_id}' was not found"),
                "validate the recipe before placement",
            ))
        })?;
    geometry_local_bounds(&recipe.geometries[geometry_index]).ok_or_else(|| {
        Box::new(scena::ScenePlacementDiagnosticV1::new(
            "missing_bounds",
            format!("$.geometries[{geometry_index}]"),
            "geometry did not produce finite authored bounds",
            "use finite mesh positions or supported primitive dimensions",
        ))
    })
}

fn geometry_local_bounds(geometry: &scena::SceneRecipeGeometryV1) -> Option<scena::Aabb> {
    if let Some(mesh) = &geometry.mesh {
        return bounds_from_points(mesh.positions.iter().copied());
    }
    let primitive = geometry.primitive.as_ref()?;
    let symmetric = |extent: scena::Vec3| scena::Aabb::new(-extent, extent);
    let bounds = match primitive.kind.as_str() {
        "box" | "wedge" => {
            let size = primitive.size.as_deref()?;
            symmetric(scena::Vec3::new(size[0] as f32, size[1] as f32, size[2] as f32) * 0.5)
        }
        "plane" => {
            let size = primitive.size.as_deref()?;
            symmetric(scena::Vec3::new(
                size[0] as f32 * 0.5,
                0.0,
                size[1] as f32 * 0.5,
            ))
        }
        "sphere" => symmetric(scena::Vec3::splat(primitive.radius? as f32)),
        "cylinder" | "cone" => {
            let radius = primitive.radius? as f32;
            symmetric(scena::Vec3::new(
                radius,
                primitive.height? as f32 * 0.5,
                radius,
            ))
        }
        "disc" => {
            let radius = primitive.radius? as f32;
            symmetric(scena::Vec3::new(radius, 0.0, radius))
        }
        "torus" => {
            let major = primitive.major_radius? as f32;
            let minor = primitive.minor_radius? as f32;
            symmetric(scena::Vec3::new(major + minor, minor, major + minor))
        }
        "line" | "arrow" => bounds_from_points([primitive.start?, primitive.end?])?,
        "polyline" => bounds_from_points(primitive.points.iter().copied())?,
        "grid" => {
            let length = primitive.length.or_else(|| {
                primitive
                    .size
                    .as_ref()
                    .and_then(|size| size.first().copied())
            })? as f32;
            symmetric(scena::Vec3::new(length * 0.5, 0.0, length * 0.5))
        }
        "axes" => {
            let length = primitive.length? as f32;
            scena::Aabb::new(scena::Vec3::ZERO, scena::Vec3::splat(length))
        }
        _ => return None,
    };
    (bounds.min.is_finite() && bounds.max.is_finite()).then_some(bounds)
}

fn bounds_from_points(points: impl IntoIterator<Item = [f64; 3]>) -> Option<scena::Aabb> {
    let mut points = points.into_iter();
    let first = points.next()?;
    let first = scena::Vec3::new(first[0] as f32, first[1] as f32, first[2] as f32);
    if !first.is_finite() {
        return None;
    }
    let mut min = first;
    let mut max = first;
    for point in points {
        let point = scena::Vec3::new(point[0] as f32, point[1] as f32, point[2] as f32);
        if !point.is_finite() {
            return None;
        }
        min = min.min(point);
        max = max.max(point);
    }
    Some(scena::Aabb::new(min, max))
}

fn authored_node_look_at_target(
    args: &PlaceCommandArgs,
    recipe: &scena::SceneRecipeV1,
    verb: &str,
) -> Result<scena::Vec3, Box<scena::ScenePlacementDiagnosticV1>> {
    if let Some(target) = args.target {
        return Ok(target);
    }
    let Some(target_import_id) = args.target_import_id.as_deref() else {
        return Err(Box::new(scena::ScenePlacementDiagnosticV1::new(
            "missing_target",
            "$.verb.look_at",
            "look_at requires --target or --target-import",
            "pass --target x,y,z or --target-import <id>",
        )));
    };
    let recipe_path = args.recipe.to_str().ok_or_else(|| {
        Box::new(scena::ScenePlacementDiagnosticV1::new(
            "invalid_recipe_path",
            "$",
            "recipe path is not valid UTF-8",
            "use a UTF-8 recipe path",
        ))
    })?;
    let runtime = load_placement_runtime(recipe_path, recipe, verb).map_err(|report| {
        Box::new(report.diagnostics.into_iter().next().unwrap_or_else(|| {
            scena::ScenePlacementDiagnosticV1::new(
                "asset_load_failed",
                "$.imports",
                "look_at target import could not be loaded",
                "fix the target import and retry",
            )
        }))
    })?;
    let target_index = runtime.import_index(target_import_id).ok_or_else(|| {
        Box::new(unknown_import_diagnostic(
            recipe,
            target_import_id,
            "$.target_import",
        ))
    })?;
    runtime.imports[target_index]
        .import
        .bounds_world(&runtime.scene)
        .map(|bounds| bounds.center())
        .ok_or_else(|| {
            Box::new(scena::ScenePlacementDiagnosticV1::new(
                "missing_bounds",
                format!(
                    "$.imports[{}].uri",
                    runtime.imports[target_index].recipe_index
                ),
                "look_at target import must have bounds",
                "choose a target point or an import with renderable bounds",
            ))
        })
}
