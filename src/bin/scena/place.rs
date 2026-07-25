use super::scena_args::{PlaceCommandArgs, PlaceTargetArg};
use super::scena_cli_error::{CliErrorKind, CliFailure};
use super::scena_input::{RecipeReadError, read_recipe_text, resolve_recipe_asset_uri};
use super::scena_output::{CliOutcome, json_outcome};
use sha2::{Digest, Sha256};

mod authored_features;
mod authored_nodes;
mod resource_uris;
use authored_nodes::{public_placement_target, run_authored_node_placement};
use resource_uris::rebase_recipe_resource_uris;

pub(crate) fn run_place_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = PlaceCommandArgs::parse(args)?;
    let policy = scena::RecipeBuildPolicy::testing();
    let text = match read_recipe_text(&args.recipe, &policy) {
        Ok(text) => text,
        Err(RecipeReadError::TooLarge(report)) => {
            return json_outcome(
                &report,
                1,
                "failed to serialize scene recipe validation report",
            );
        }
        Err(RecipeReadError::Io(error)) => {
            return Err(CliFailure::new(
                CliErrorKind::InputNotFound,
                format!("failed to read recipe '{}': {error}", args.recipe.display()),
            ));
        }
    };
    let recipe = match scena::parse_valid_scene_recipe_json(&text) {
        Ok(recipe) => recipe,
        Err(report) => {
            return json_outcome(
                &report,
                1,
                "failed to serialize scene recipe validation report",
            );
        }
    };
    let recipe_path = args
        .recipe
        .to_str()
        .ok_or_else(|| format!("recipe path '{}' is not valid UTF-8", args.recipe.display()))?;
    let verb = match args.verb.as_str() {
        "fit-to-size" => "fit_to_size".to_owned(),
        other => other.to_owned(),
    };
    if matches!(args.target_subject, PlaceTargetArg::Node(_)) {
        return run_authored_node_placement(&args, &text, recipe, verb);
    }
    let import_id = args.target_subject.id();
    let runtime = match load_placement_runtime(recipe_path, &recipe, &verb) {
        Ok(runtime) => runtime,
        Err(report) => return json_outcome(&*report, 1, "failed to serialize placement result"),
    };
    let Some(source_index) = runtime.import_index(import_id) else {
        let report = scena::ScenePlacementResultV1::failure(
            import_id,
            verb,
            unknown_import_diagnostic(&recipe, import_id, "$.imports"),
        );
        return json_outcome(&report, 1, "failed to serialize placement result");
    };

    let result = match verb.as_str() {
        "center" => place_bounds(&runtime, source_index, verb, |bounds, transform| {
            Ok(scena::placement_center_transform(
                bounds,
                transform,
                args.target.unwrap_or(scena::Vec3::ZERO),
            ))
        }),
        "ground" => place_bounds(&runtime, source_index, verb, |bounds, transform| {
            Ok(scena::placement_ground_transform(
                bounds,
                transform,
                args.ground_y.unwrap_or(0.0),
            ))
        }),
        "fit_to_size" => place_bounds(&runtime, source_index, verb, |bounds, transform| {
            scena::placement_fit_to_size_transform(bounds, transform, args.min_size, args.max_size)
        }),
        "look_at" => {
            let current = source_world_transform(&runtime, source_index);
            match placement_target_point(&runtime, &args) {
                Ok(target) => match scena::placement_look_at_transform(
                    current,
                    target,
                    args.up.unwrap_or(scena::Vec3::Y),
                ) {
                    Ok(transform) => scena::ScenePlacementResultV1::success(
                        runtime.imports[source_index].id.clone(),
                        verb,
                        transform,
                    ),
                    Err(error) => scena::ScenePlacementResultV1::failure(
                        runtime.imports[source_index].id.clone(),
                        verb,
                        *error,
                    ),
                },
                Err(diagnostic) => scena::ScenePlacementResultV1::failure(
                    runtime.imports[source_index].id.clone(),
                    verb,
                    *diagnostic,
                ),
            }
        }
        "align_to_anchor" => {
            authored_features::place_authored_feature(&runtime, &args, source_index, verb, true)
        }
        "place_on" => {
            authored_features::place_authored_feature(&runtime, &args, source_index, verb, false)
        }
        _ => scena::ScenePlacementResultV1::failure(
            runtime.imports[source_index].id.clone(),
            verb,
            scena::ScenePlacementDiagnosticV1::new(
                "unknown_verb",
                "$.verb",
                format!("placement verb '{}' is not supported", args.verb),
                "use center, ground, fit_to_size, look_at, align_to_anchor, or place_on",
            )
            .with_suggestion("center"),
        ),
    };
    if args.apply {
        return emit_recipe_patch(&args, &text, recipe, result);
    }
    let exit_code = if result.ok { 0 } else { 1 };
    json_outcome(&result, exit_code, "failed to serialize placement result")
}

fn emit_recipe_patch(
    args: &PlaceCommandArgs,
    source_text: &str,
    mut recipe: scena::SceneRecipeV1,
    placement: scena::ScenePlacementResultV1,
) -> Result<CliOutcome, CliFailure> {
    let target_id = args.target_subject.id();
    let public_target = public_placement_target(&args.target_subject);
    let source_sha256 = sha256_hex(source_text.as_bytes());
    let source_path = args.recipe.display().to_string();
    if let Some(expected) = args.expected_source_sha256.as_deref()
        && expected != source_sha256
    {
        let report = scena::SceneRecipePatchResultV1::failure_for_target(
            source_path,
            source_sha256.clone(),
            public_target.clone(),
            placement.verb,
            scena::ScenePlacementDiagnosticV1::new(
                "stale_source",
                "$",
                format!("recipe source digest changed; expected {expected}, found {source_sha256}"),
                "reload the recipe, recompute placement, and apply against the new digest",
            ),
        );
        return json_outcome(&report, 1, "failed to serialize recipe patch result");
    }
    let Some(transform) = placement.transform else {
        let report = scena::SceneRecipePatchResultV1::failure_for_target(
            source_path,
            source_sha256,
            public_target.clone(),
            placement.verb,
            placement.diagnostics.into_iter().next().unwrap_or_else(|| {
                scena::ScenePlacementDiagnosticV1::new(
                    "placement_failed",
                    "$.imports",
                    "placement failed without a transform",
                    "fix the placement diagnostics and retry",
                )
            }),
        );
        return json_outcome(&report, 1, "failed to serialize recipe patch result");
    };
    let (previous_transform, semantic_path) = match &args.target_subject {
        PlaceTargetArg::Import(_) => {
            let Some((index, import)) = recipe
                .imports
                .iter_mut()
                .enumerate()
                .find(|(_, import)| import.id == target_id)
            else {
                return Err(vanished_after_validation("import"));
            };
            let previous = import
                .transform
                .as_ref()
                .map(scena::Transform::try_from)
                .transpose()
                .map_err(|error| {
                    format!("validated import transform failed to resolve: {error}")
                })?;
            import.transform = Some(scena::SceneRecipeTransformV1::from(transform));
            (previous, format!("$.imports[{index}].transform"))
        }
        PlaceTargetArg::Node(_) => {
            let Some((index, node)) = recipe
                .nodes
                .iter_mut()
                .enumerate()
                .find(|(_, node)| node.id == target_id)
            else {
                return Err(vanished_after_validation("node"));
            };
            let previous = node
                .transform
                .as_ref()
                .map(scena::Transform::try_from)
                .transpose()
                .map_err(|error| format!("validated node transform failed to resolve: {error}"))?;
            node.transform = Some(scena::SceneRecipeTransformV1::from(transform));
            (previous, format!("$.nodes[{index}].transform"))
        }
    };
    rebase_recipe_resource_uris(&source_path, &mut recipe);
    let updated_recipe = serde_json::to_value(&recipe)
        .map_err(|error| format!("failed to serialize updated recipe: {error}"))?;
    let report = scena::SceneRecipePatchResultV1::success_for_target(
        scena::SceneRecipePatchSuccessInputV1 {
            source_path,
            source_sha256,
            import_id: target_id.to_owned(),
            verb: placement.verb,
            previous_transform,
            transform,
            updated_recipe,
            semantic_change: scena::SceneRecipeSemanticChangeV1::transform(
                semantic_path,
                previous_transform,
                transform,
            ),
        },
        public_target,
    );
    json_outcome(&report, 0, "failed to serialize recipe patch result")
}

/// A placement target that validation just resolved is gone. Only reachable if
/// the recipe mutated between validation and patch emission, so it is an
/// internal invariant failure, not something the caller can fix.
fn vanished_after_validation(kind: &str) -> CliFailure {
    CliFailure::new(
        CliErrorKind::Internal,
        format!("validated placement {kind} disappeared before patch emission"),
    )
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

struct PlacementRuntime {
    scene: scena::Scene,
    imports: Vec<PlacementRuntimeImport>,
}

struct PlacementRuntimeImport {
    id: String,
    recipe_index: usize,
    asset: scena::SceneAsset,
    import: scena::SceneImport,
    transform: scena::Transform,
}

impl PlacementRuntime {
    fn import_index(&self, import_id: &str) -> Option<usize> {
        self.imports
            .iter()
            .position(|import| import.id == import_id)
    }
}

fn place_bounds(
    runtime: &PlacementRuntime,
    source_index: usize,
    verb: String,
    compute: impl FnOnce(
        scena::Aabb,
        scena::Transform,
    ) -> Result<scena::Transform, Box<scena::ScenePlacementDiagnosticV1>>,
) -> scena::ScenePlacementResultV1 {
    let source = &runtime.imports[source_index];
    let Some(bounds) = source.asset.bounds() else {
        return missing_bounds_report(source, &verb);
    };
    match compute(bounds, source.transform) {
        Ok(transform) => scena::ScenePlacementResultV1::success(source.id.clone(), verb, transform),
        Err(error) => scena::ScenePlacementResultV1::failure(source.id.clone(), verb, *error),
    }
}

fn load_placement_runtime(
    recipe_path: &str,
    recipe: &scena::SceneRecipeV1,
    verb: &str,
) -> Result<PlacementRuntime, Box<scena::ScenePlacementResultV1>> {
    let assets = scena::Assets::new();
    let mut scene = scena::Scene::new();
    let mut imports = Vec::new();
    for (recipe_index, import) in recipe.imports.iter().enumerate() {
        let asset_uri = resolve_recipe_asset_uri(recipe_path, &import.uri);
        let scene_asset =
            pollster::block_on(assets.load_scene(asset_uri.as_str())).map_err(|error| {
                Box::new(scena::ScenePlacementResultV1::failure(
                    import.id.clone(),
                    verb.to_owned(),
                    scena::ScenePlacementDiagnosticV1::new(
                        "asset_load_failed",
                        format!("$.imports[{recipe_index}].uri"),
                        format!("failed to load placement asset '{}': {error}", import.uri),
                        "fix the recipe uri or run validate-recipe before placement",
                    ),
                ))
            })?;
        let scene_import = scene.instantiate(&scene_asset).map_err(|error| {
            Box::new(scena::ScenePlacementResultV1::failure(
                import.id.clone(),
                verb.to_owned(),
                scena::ScenePlacementDiagnosticV1::new(
                    "asset_instantiate_failed",
                    format!("$.imports[{recipe_index}].uri"),
                    format!(
                        "failed to instantiate placement asset '{}': {error}",
                        import.uri
                    ),
                    "fix authored anchors/connectors in the asset before placement",
                ),
            ))
        })?;
        let transform = match import.transform.as_ref() {
            Some(transform) => match scena::Transform::try_from(transform) {
                Ok(transform) => transform,
                Err(error) => {
                    return Err(Box::new(scena::ScenePlacementResultV1::failure(
                        import.id.clone(),
                        verb.to_owned(),
                        scena::ScenePlacementDiagnosticV1::new(
                            "invalid_transform",
                            format!("$.imports[{recipe_index}].transform"),
                            error.to_string(),
                            "use a canonical kind:raw or kind:trs local transform",
                        ),
                    )));
                }
            },
            None => scena::Transform::IDENTITY,
        };
        for root in scene_import.roots() {
            scene.set_transform(*root, transform).map_err(|error| {
                Box::new(scena::ScenePlacementResultV1::failure(
                    import.id.clone(),
                    verb.to_owned(),
                    scena::ScenePlacementDiagnosticV1::new(
                        "invalid_import_root",
                        format!("$.imports[{recipe_index}].transform"),
                        format!("failed to apply import transform: {error}"),
                        "use a valid recipe import transform",
                    ),
                ))
            })?;
        }
        imports.push(PlacementRuntimeImport {
            id: import.id.clone(),
            recipe_index,
            asset: scene_asset,
            import: scene_import,
            transform,
        });
    }
    Ok(PlacementRuntime { scene, imports })
}

fn missing_bounds_report(
    import: &PlacementRuntimeImport,
    verb: &str,
) -> scena::ScenePlacementResultV1 {
    scena::ScenePlacementResultV1::failure(
        import.id.clone(),
        verb.to_owned(),
        scena::ScenePlacementDiagnosticV1::new(
            "missing_bounds",
            format!("$.imports[{}].uri", import.recipe_index),
            "placement requires an asset with renderable bounds",
            "use an asset containing mesh, primitive, or instance bounds",
        ),
    )
}

fn unknown_import_diagnostic(
    recipe: &scena::SceneRecipeV1,
    import_id: &str,
    path: &str,
) -> scena::ScenePlacementDiagnosticV1 {
    if recipe.nodes.iter().any(|node| node.id == import_id) {
        return scena::ScenePlacementDiagnosticV1::new(
            "wrong_target_namespace",
            path,
            format!("'{import_id}' is an authored node, not an import"),
            format!("use --node {import_id}"),
        )
        .with_candidates(vec![format!("--node {import_id}")]);
    }
    let mut diagnostic = scena::ScenePlacementDiagnosticV1::new(
        "unknown_import",
        path,
        format!("recipe import '{import_id}' was not found"),
        "pass an import id declared in the recipe",
    );
    let candidates = scena::nearest_name_candidates(
        import_id,
        recipe.imports.iter().map(|import| import.id.as_str()),
        3,
    );
    if let Some(first) = candidates.first() {
        diagnostic = diagnostic.with_suggestion(first.clone());
    }
    diagnostic.with_candidates(candidates)
}

fn source_world_transform(runtime: &PlacementRuntime, source_index: usize) -> scena::Transform {
    runtime.imports[source_index]
        .import
        .roots()
        .first()
        .and_then(|root| runtime.scene.world_transform(*root))
        .unwrap_or(runtime.imports[source_index].transform)
}

fn placement_target_point(
    runtime: &PlacementRuntime,
    args: &PlaceCommandArgs,
) -> Result<scena::Vec3, Box<scena::ScenePlacementDiagnosticV1>> {
    if let Some(target) = args.target {
        return Ok(target);
    }
    let Some(target_import_id) = &args.target_import_id else {
        return Err(Box::new(scena::ScenePlacementDiagnosticV1::new(
            "missing_target",
            "$.verb.look_at",
            "look_at requires --target or --target-import",
            "pass --target x,y,z or --target-import <id>",
        )));
    };
    let Some(target_index) = runtime.import_index(target_import_id) else {
        return Err(Box::new(scena::ScenePlacementDiagnosticV1::new(
            "unknown_import",
            "$.target_import",
            format!("target import '{target_import_id}' was not found"),
            "pass --target-import with one of the recipe import ids",
        )));
    };
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
