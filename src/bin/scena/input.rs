#[cfg(all(feature = "inspection", feature = "scene-host"))]
use std::collections::BTreeMap;
#[cfg(feature = "inspection")]
use std::fs;
use std::path::Path;
#[cfg(feature = "inspection")]
use std::path::PathBuf;

#[cfg(feature = "inspection")]
use super::{CliOutcome, json_outcome};

#[cfg(feature = "inspection")]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedSceneInput {
    pub(crate) asset: String,
    pub(crate) transform: Option<scena::Transform>,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) recipe_path: Option<String>,
    pub(crate) recipe: Option<scena::SceneRecipeV1>,
}

#[cfg(feature = "inspection")]
pub(crate) fn resolve_scene_input(input: &str) -> Result<ResolvedSceneInput, CliOutcome> {
    match try_load_recipe(input)? {
        Some(recipe) => {
            let import = recipe
                .imports
                .first()
                .expect("validated scene recipe contains an import");
            let asset = resolve_recipe_asset_uri(input, &import.uri);
            Ok(ResolvedSceneInput {
                asset,
                transform: import.transform,
                width: recipe.capture.as_ref().map(|capture| capture.width),
                height: recipe.capture.as_ref().map(|capture| capture.height),
                recipe_path: Some(input.to_owned()),
                recipe: Some(recipe),
            })
        }
        None => Ok(ResolvedSceneInput {
            asset: input.to_owned(),
            transform: None,
            width: None,
            height: None,
            recipe_path: None,
            recipe: None,
        }),
    }
}

#[cfg(feature = "inspection")]
impl ResolvedSceneInput {
    pub(crate) fn has_scene_host_directives(&self) -> bool {
        self.recipe
            .as_ref()
            .is_some_and(scene_recipe_has_scene_host_directives)
    }
}

#[cfg(feature = "inspection")]
pub(crate) fn scene_recipe_has_scene_host_directives(recipe: &scena::SceneRecipeV1) -> bool {
    recipe.section_box.is_some()
        || !recipe.measurements.is_empty()
        || !recipe.callouts.is_empty()
        || recipe.exploded_view.is_some()
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
pub(crate) async fn scene_host_from_resolved_recipe(
    input: &ResolvedSceneInput,
    width: u32,
    height: u32,
) -> Result<scena::SceneHostCore, String> {
    let recipe = input
        .recipe
        .as_ref()
        .ok_or_else(|| "scene-host recipe rendering requires a scene recipe input".to_string())?;
    let recipe_path = input.recipe_path.as_deref().unwrap_or(&input.asset);
    let mut host = scena::SceneHostCore::headless(width, height)
        .map_err(|error| format!("failed to create SceneHost renderer: {error}"))?;
    let mut imports = BTreeMap::new();

    for import in &recipe.imports {
        let asset = resolve_recipe_asset_uri(recipe_path, &import.uri);
        let import_handle = host
            .instantiate_url(scena::AssetPath::from(asset.as_str()))
            .await
            .map_err(|error| format!("failed to load recipe import '{}': {error}", import.id))?;
        let roots = host.import_roots(import_handle).map_err(|error| {
            format!(
                "failed to inspect recipe import roots '{}': {error}",
                import.id
            )
        })?;
        if let Some(transform) = import.transform {
            for root in &roots {
                host.set_transform(*root, transform).map_err(|error| {
                    format!(
                        "failed to apply transform for recipe import '{}': {error}",
                        import.id
                    )
                })?;
            }
        }
        imports.insert(import.id.clone(), RecipeImportHandles { roots });
    }

    apply_recipe_section_box(&mut host, recipe, &imports)?;
    apply_recipe_measurements(&mut host, recipe)?;
    apply_recipe_callouts(&mut host, recipe, &imports)?;
    apply_recipe_exploded_view(&mut host, recipe, &imports)?;
    host.frame_all_with_overlays()
        .map_err(|error| format!("failed to frame recipe scene including overlays: {error}"))?;
    Ok(host)
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
struct RecipeImportHandles {
    roots: Vec<u64>,
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn apply_recipe_section_box(
    host: &mut scena::SceneHostCore,
    recipe: &scena::SceneRecipeV1,
    imports: &BTreeMap<String, RecipeImportHandles>,
) -> Result<(), String> {
    let Some(section_box) = &recipe.section_box else {
        return Ok(());
    };
    let bounds = import_bounds(host, imports, &section_box.import)?;
    host.set_section_box_json(
        bounds,
        section_box.margin,
        section_box.inverted,
        section_box.helper_wireframe,
    )
    .map(|_| ())
    .map_err(|error| format!("failed to apply recipe section_box: {error}"))
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn apply_recipe_measurements(
    host: &mut scena::SceneHostCore,
    recipe: &scena::SceneRecipeV1,
) -> Result<(), String> {
    for measurement in &recipe.measurements {
        if measurement.kind != "distance" {
            return Err(format!(
                "unsupported recipe measurement kind '{}'",
                measurement.kind
            ));
        }
        host.add_distance_measurement_json(
            &measurement.id,
            scena::Vec3::from_array(measurement.start),
            scena::Vec3::from_array(measurement.end),
            measurement.label.as_deref(),
            measurement.unit.as_deref().unwrap_or("unit"),
            measurement.precision.unwrap_or(2),
        )
        .map(|_| ())
        .map_err(|error| {
            format!(
                "failed to apply recipe measurement '{}': {error}",
                measurement.id
            )
        })?;
    }
    Ok(())
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn apply_recipe_callouts(
    host: &mut scena::SceneHostCore,
    recipe: &scena::SceneRecipeV1,
    imports: &BTreeMap<String, RecipeImportHandles>,
) -> Result<(), String> {
    for callout in &recipe.callouts {
        match &callout.target {
            scena::SceneRecipeCalloutTargetV1::ImportRoot {
                import,
                local_offset,
            } => {
                let root = first_import_root(imports, import)?;
                host.add_node_callout(
                    &callout.id,
                    root,
                    *local_offset,
                    callout.label_offset,
                    &callout.text,
                )
            }
            scena::SceneRecipeCalloutTargetV1::World { position } => {
                host.add_world_callout(&callout.id, *position, callout.label_offset, &callout.text)
            }
        }
        .map(|_| ())
        .map_err(|error| format!("failed to apply recipe callout '{}': {error}", callout.id))?;
    }
    Ok(())
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn apply_recipe_exploded_view(
    host: &mut scena::SceneHostCore,
    recipe: &scena::SceneRecipeV1,
    imports: &BTreeMap<String, RecipeImportHandles>,
) -> Result<(), String> {
    let Some(exploded) = &recipe.exploded_view else {
        return Ok(());
    };
    let root = first_import_root(imports, &exploded.import)?;
    let mode = match exploded.mode {
        scena::SceneRecipeExplodedViewModeV1::DirectChildren => {
            scena::SceneHostExplodedViewModeV1::DirectChildren
        }
        scena::SceneRecipeExplodedViewModeV1::HierarchyDepth => {
            scena::SceneHostExplodedViewModeV1::HierarchyDepth
        }
        scena::SceneRecipeExplodedViewModeV1::Axis => scena::SceneHostExplodedViewModeV1::Axis,
    };
    let patch = host
        .exploded_view_patch(
            root,
            scena::SceneHostExplodedViewOptionsV1 {
                mode,
                axis: exploded.axis,
                factor: exploded.factor,
                distance: exploded.distance,
                duration_seconds: None,
                easing: scena::SceneHostEasing::Linear,
            },
        )
        .map_err(|error| format!("failed to build recipe exploded_view patch: {error}"))?;
    host.apply_patch(&patch)
        .map(|_| ())
        .map_err(|error| format!("failed to apply recipe exploded_view patch: {error}"))
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn import_bounds(
    host: &mut scena::SceneHostCore,
    imports: &BTreeMap<String, RecipeImportHandles>,
    import_id: &str,
) -> Result<scena::Aabb, String> {
    let handles = imports
        .get(import_id)
        .ok_or_else(|| format!("recipe references unknown import '{import_id}'"))?;
    let mut combined = None;
    for root in &handles.roots {
        let Some(bounds) = host.node_world_bounds(*root).map_err(|error| {
            format!("failed to compute bounds for import '{import_id}': {error}")
        })?
        else {
            continue;
        };
        combined = Some(combined.map_or(bounds, |current: scena::Aabb| current.union(bounds)));
    }
    combined.ok_or_else(|| format!("recipe import '{import_id}' has no renderable bounds"))
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
fn first_import_root(
    imports: &BTreeMap<String, RecipeImportHandles>,
    import_id: &str,
) -> Result<u64, String> {
    imports
        .get(import_id)
        .and_then(|handles| handles.roots.first().copied())
        .ok_or_else(|| format!("recipe import '{import_id}' has no root node"))
}

#[cfg(feature = "inspection")]
pub(crate) fn viewer_builder(
    asset: &str,
    width: u32,
    height: u32,
    transform: Option<scena::Transform>,
) -> scena::HeadlessGltfViewerBuilder {
    let builder = scena::headless_gltf_viewer(asset).size(width, height);
    if let Some(transform) = transform {
        builder.with_import_transform(transform)
    } else {
        builder
    }
}

#[cfg(feature = "inspection")]
pub(crate) fn render_introspection_options(detail: bool) -> scena::RenderIntrospectionOptions {
    if detail {
        scena::RenderIntrospectionOptions::detail()
    } else {
        scena::RenderIntrospectionOptions::summary()
    }
}

#[cfg(feature = "inspection")]
pub(crate) fn appearance_introspection_options(
    detail: bool,
) -> scena::AppearanceIntrospectionOptions {
    if detail {
        scena::AppearanceIntrospectionOptions::detail()
    } else {
        scena::AppearanceIntrospectionOptions::summary()
    }
}

#[cfg(feature = "inspection")]
pub(crate) fn asset_doctor_outcome_or_error(
    asset: &str,
    command: &str,
    error: String,
) -> Result<CliOutcome, String> {
    let report = pollster::block_on(scena::Assets::new().doctor_asset_path(asset));
    if !report.ok {
        return json_outcome(
            &report,
            1,
            "failed to serialize asset doctor report for failed command",
        );
    }
    Err(format!("failed to {command} '{asset}': {error}"))
}

#[cfg(feature = "inspection")]
pub(crate) fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            format!("failed to create directory '{}': {error}", parent.display())
        })?;
    }
    Ok(())
}

#[cfg(feature = "inspection")]
pub(crate) fn capture_descriptor_path(png_path: &Path) -> PathBuf {
    let stem = png_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("capture");
    png_path.with_file_name(format!("{stem}.capture.json"))
}

#[cfg(feature = "inspection")]
pub(crate) fn path_for_json(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(feature = "inspection")]
fn try_load_recipe(input: &str) -> Result<Option<scena::SceneRecipeV1>, CliOutcome> {
    let path = Path::new(input);
    let Ok(text) = fs::read_to_string(path) else {
        return Ok(None);
    };
    let is_recipe_path = input.ends_with(".recipe.json");
    let parsed = serde_json::from_str::<serde_json::Value>(&text);
    let is_recipe_schema = parsed
        .as_ref()
        .ok()
        .and_then(|value| value.get("schema"))
        .and_then(serde_json::Value::as_str)
        == Some(scena::SCENE_RECIPE_SCHEMA_V1);
    if !is_recipe_path && !is_recipe_schema {
        return Ok(None);
    }
    match scena::parse_valid_scene_recipe_json(&text) {
        Ok(recipe) => Ok(Some(recipe)),
        Err(report) => {
            let outcome = json_outcome(
                &report,
                1,
                "failed to serialize scene recipe validation report",
            )
            .expect("scene recipe validation report serializes");
            Err(outcome)
        }
    }
}

pub(crate) fn resolve_recipe_asset_uri(recipe_path: &str, uri: &str) -> String {
    let uri_path = Path::new(uri);
    if uri_path.is_absolute() || uri.contains("://") || uri.starts_with("data:") {
        return uri.to_owned();
    }
    let relative_to_recipe = Path::new(recipe_path)
        .parent()
        .map(|parent| parent.join(uri));
    if let Some(path) = relative_to_recipe.filter(|path| path.exists()) {
        return path.display().to_string();
    }
    uri.to_owned()
}
