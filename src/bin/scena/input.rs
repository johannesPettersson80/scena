use std::fs;
use std::io::Read;
use std::path::Path;
#[cfg(feature = "inspection")]
use std::path::PathBuf;

#[cfg(feature = "inspection")]
use super::scena_output::{CliOutcome, json_outcome};

pub(crate) enum RecipeReadError {
    Io(String),
    TooLarge(scena::SceneRecipeValidationReportV1),
}

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
        || !recipe.instance_sets.is_empty()
        || !recipe.labels.is_empty()
        || !recipe.clipping_planes.is_empty()
        || !recipe.animations.is_empty()
        || !recipe.cameras.is_empty()
        || !recipe.lights.is_empty()
        || recipe.scene.is_some()
        || recipe.render.is_some()
        || recipe.expect.is_some()
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
    use_gpu: bool,
) -> Result<scena::SceneHostCore, String> {
    Ok(
        scene_host_build_from_resolved_recipe(input, width, height, use_gpu)
            .await?
            .host,
    )
}

#[cfg(all(feature = "inspection", feature = "scene-host"))]
pub(crate) async fn scene_host_build_from_resolved_recipe(
    input: &ResolvedSceneInput,
    width: u32,
    height: u32,
    use_gpu: bool,
) -> Result<scena::SceneHostRecipeBuild, String> {
    let recipe = input
        .recipe
        .as_ref()
        .ok_or_else(|| "scene-host recipe rendering requires a scene recipe input".to_string())?;
    let recipe_path = input.recipe_path.as_deref().unwrap_or(&input.asset);
    let recipe_text = serde_json::to_string(recipe)
        .map_err(|error| format!("failed to serialize scene recipe for build: {error}"))?;
    let policy = scena::RecipeBuildPolicy::testing();
    let mut build = if use_gpu {
        scena::SceneHostCore::build_recipe_json_gpu(recipe_path, &recipe_text, policy).await
    } else {
        scena::SceneHostCore::build_recipe_json(recipe_path, &recipe_text, policy).await
    }
    .map_err(|manifest| {
        serde_json::to_string_pretty(&manifest)
            .unwrap_or_else(|error| format!("failed to serialize build failure manifest: {error}"))
    })?;
    build
        .host
        .resize(width as f32, height as f32, 1.0)
        .map_err(|error| format!("failed to size recipe SceneHost renderer: {error}"))?;
    if !recipe.cameras.iter().any(|camera| camera.active) {
        build
            .host
            .frame_all_with_overlays()
            .map_err(|error| format!("failed to frame recipe scene including overlays: {error}"))?;
    }
    Ok(build)
}

#[cfg(feature = "inspection")]
pub(crate) fn viewer_builder(
    asset: &str,
    width: u32,
    height: u32,
    transform: Option<scena::Transform>,
    use_gpu: bool,
) -> scena::HeadlessGltfViewerBuilder {
    let builder = scena::headless_gltf_viewer(asset).size(width, height);
    let builder = if use_gpu {
        builder.with_headless_gpu()
    } else {
        builder
    };
    if let Some(transform) = transform {
        builder.with_import_transform(transform)
    } else {
        builder
    }
}

#[cfg(feature = "inspection")]
pub(crate) fn gpu_requested_from_env() -> bool {
    std::env::var("SCENA_USE_GPU").is_ok_and(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
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
    let is_recipe_path = input.ends_with(".recipe.json");
    let policy = scena::RecipeBuildPolicy::testing();
    let text = match read_recipe_text(path, &policy) {
        Ok(text) => text,
        Err(RecipeReadError::Io(_)) => return Ok(None),
        Err(RecipeReadError::TooLarge(report)) if is_recipe_path => {
            return Err(json_outcome(
                &report,
                1,
                "failed to serialize scene recipe validation report",
            )
            .expect("scene recipe validation report serializes"));
        }
        Err(RecipeReadError::TooLarge(_)) => return Ok(None),
    };
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

pub(crate) fn read_recipe_text(
    path: &Path,
    policy: &scena::RecipeBuildPolicy,
) -> Result<String, RecipeReadError> {
    let max_bytes = policy.max_recipe_bytes();
    if let Ok(metadata) = fs::metadata(path)
        && metadata.is_file()
    {
        let byte_len = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
        if byte_len > max_bytes {
            return Err(RecipeReadError::TooLarge(scena::recipe_too_large_report(
                byte_len, max_bytes,
            )));
        }
    }

    let file = fs::File::open(path).map_err(|error| RecipeReadError::Io(error.to_string()))?;
    let mut reader = file.take(max_bytes.saturating_add(1) as u64);
    let mut text = String::new();
    reader
        .read_to_string(&mut text)
        .map_err(|error| RecipeReadError::Io(error.to_string()))?;
    if text.len() > max_bytes {
        return Err(RecipeReadError::TooLarge(scena::recipe_too_large_report(
            text.len(),
            max_bytes,
        )));
    }
    Ok(text)
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
