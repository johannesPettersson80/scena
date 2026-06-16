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
            let asset = recipe
                .imports
                .first()
                .map(|import| resolve_recipe_asset_uri(input, &import.uri))
                .unwrap_or_else(|| input.to_owned());
            Ok(ResolvedSceneInput {
                asset,
                transform: recipe.imports.first().and_then(|import| import.transform),
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
    !recipe.colors.is_empty()
        || !recipe.geometries.is_empty()
        || !recipe.materials.is_empty()
        || !recipe.nodes.is_empty()
        || !recipe.cameras.is_empty()
        || !recipe.lights.is_empty()
        || recipe.section_box.is_some()
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
    let recipe_text = serde_json::to_string(recipe)
        .map_err(|error| format!("failed to serialize scene recipe for build: {error}"))?;
    let build = scena::SceneHostCore::build_recipe_json(
        recipe_path,
        &recipe_text,
        scena::RecipeBuildPolicy::testing(),
    )
    .await
    .map_err(|manifest| {
        serde_json::to_string_pretty(&manifest)
            .unwrap_or_else(|error| format!("failed to serialize build failure manifest: {error}"))
    })?;
    let mut host = build.host;
    host.resize(width as f32, height as f32, 1.0)
        .map_err(|error| format!("failed to size recipe SceneHost renderer: {error}"))?;
    if !recipe.cameras.iter().any(|camera| camera.active) {
        host.frame_all_with_overlays()
            .map_err(|error| format!("failed to frame recipe scene including overlays: {error}"))?;
    }
    Ok(host)
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
