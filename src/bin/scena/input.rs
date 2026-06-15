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
            })
        }
        None => Ok(ResolvedSceneInput {
            asset: input.to_owned(),
            transform: None,
            width: None,
            height: None,
        }),
    }
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
