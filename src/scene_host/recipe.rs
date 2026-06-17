use std::collections::BTreeMap;

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    RecipeBuildPolicy, SCENE_RECIPE_BUILD_SCHEMA_V1, SceneRecipeBuildImportV1,
    SceneRecipeBuildResourceV1, SceneRecipeBuildSkippedV1, SceneRecipeBuildTargetV1,
    SceneRecipeBuildV1, SceneRecipeDiagnosticV1, build_diagnostic, parse_valid_scene_recipe_json,
};
use crate::{AssetPath, Assets, Renderer, SurfaceViewport};

mod authoring;
mod overlays;
mod policy;
mod setup;

use authoring::{
    AuthoredNodeResources, InstanceSetResources, build_authored_cameras,
    build_authored_clipping_planes, build_authored_geometries, build_authored_instance_sets,
    build_authored_labels, build_authored_lights, build_authored_materials, build_authored_nodes,
};
use overlays::apply_recipe_overlays;
use policy::asset_policy_diagnostics;
use setup::{apply_render_setup, apply_scene_setup, renderer_options_from_recipe};

#[derive(Debug)]
pub struct SceneHostRecipeBuild<F = DefaultAssetFetcher> {
    pub host: SceneHostCore<F>,
    pub manifest: SceneRecipeBuildV1,
}

impl SceneHostCore<DefaultAssetFetcher> {
    pub async fn build_recipe_json(
        recipe_path: impl AsRef<str>,
        text: &str,
        policy: RecipeBuildPolicy,
    ) -> Result<SceneHostRecipeBuild<DefaultAssetFetcher>, SceneRecipeBuildV1> {
        let recipe_path = recipe_path.as_ref();
        let recipe = match parse_valid_scene_recipe_json(text) {
            Ok(recipe) => recipe,
            Err(report) => return Err(build_manifest(report.diagnostics, Vec::new())),
        };

        let mut diagnostics = Vec::new();
        let mut skipped = Vec::new();
        if recipe.imports.len() > policy.max_imports() {
            diagnostics.push(error_diagnostic(
                "$.imports",
                "policy_violation",
                format!(
                    "recipe imports {} assets, exceeding RecipeBuildPolicy max_imports {}",
                    recipe.imports.len(),
                    policy.max_imports()
                ),
                "split the recipe or raise the operator-owned max_imports policy",
            ));
        }
        if let Some(capture) = &recipe.capture {
            let output_pixels = u64::from(capture.width) * u64::from(capture.height);
            if output_pixels > policy.max_output_pixels() {
                diagnostics.push(error_diagnostic(
                    "$.capture",
                    "policy_violation",
                    format!(
                        "capture requests {output_pixels} pixels, exceeding RecipeBuildPolicy max_output_pixels {}",
                        policy.max_output_pixels()
                    ),
                    "lower capture width/height or raise the operator-owned max_output_pixels policy",
                ));
            }
        }
        if has_errors(&diagnostics) {
            return Err(build_manifest(diagnostics, skipped));
        }

        let (width, height) = recipe
            .capture
            .as_ref()
            .map(|capture| (capture.width, capture.height))
            .unwrap_or((800, 600));
        let mut host = match recipe_headless_host(
            width,
            height,
            renderer_options_from_recipe(recipe.render.as_ref()),
        ) {
            Ok(host) => host,
            Err(error) => {
                diagnostics.push(scene_host_error_diagnostic("$", "build_failed", error));
                return Err(build_manifest(diagnostics, skipped));
            }
        };
        apply_render_setup(&mut host, recipe.render.as_ref());
        let mut imports = Vec::new();
        let mut geometries = Vec::new();
        let mut materials = Vec::new();
        let mut nodes = Vec::new();
        let mut cameras = Vec::new();
        let mut lights = Vec::new();

        for (index, import) in recipe.imports.iter().enumerate() {
            let import_path = format!("$.imports[{index}]");
            let resolved_uri = match policy.resolve_import_uri(
                recipe_path,
                &import.uri,
                format!("{import_path}.uri"),
            ) {
                Ok(uri) => uri,
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    continue;
                }
            };

            let report =
                match host
                    .assets
                    .load_scene_with_report(AssetPath::from(resolved_uri.as_str()))
                    .await
                {
                    Ok(report) => report,
                    Err(error) if import.optional => {
                        diagnostics.push(build_diagnostic(
                            "optional_import_skipped",
                            "warning",
                            &import_path,
                            format!(
                                "optional import '{}' could not be loaded: {error}",
                                import.id
                            ),
                            "the import was marked optional, so the build continues without it",
                            None,
                            false,
                        ));
                        skipped.push(SceneRecipeBuildSkippedV1 {
                            path: import_path,
                            id: import.id.clone(),
                            kind: "import".to_owned(),
                            reason: error.to_string(),
                        });
                        continue;
                    }
                    Err(error) => {
                        diagnostics.push(error_diagnostic(
                        &import_path,
                        "import_load_failed",
                        format!("required import '{}' could not be loaded: {error}", import.id),
                        "fix the import uri or mark it optional only if it is allowed to be absent",
                    ));
                        continue;
                    }
                };

            let diagnostic_start = diagnostics.len();
            diagnostics.extend(asset_policy_diagnostics(
                &policy,
                &host,
                &report,
                &import_path,
            ));
            if has_errors(&diagnostics[diagnostic_start..]) {
                continue;
            }

            let import_handle =
                match host.instantiate_scene_asset_under(host.scene.root(), report.asset()) {
                    Ok(handle) => handle,
                    Err(error) => {
                        diagnostics.push(scene_host_error_diagnostic(
                            &import_path,
                            "import_instantiate_failed",
                            error,
                        ));
                        continue;
                    }
                };
            let asset_report = report.to_schema_report();
            host.emit_asset_load_events(import_handle, &asset_report);

            let root_handles = match host.import_roots(import_handle) {
                Ok(handles) => handles,
                Err(error) => {
                    diagnostics.push(scene_host_error_diagnostic(
                        &import_path,
                        "import_roots_failed",
                        error,
                    ));
                    continue;
                }
            };
            if let Some(transform) = import.transform {
                for root in &root_handles {
                    if let Err(error) = host.set_transform(*root, transform) {
                        diagnostics.push(scene_host_error_diagnostic(
                            &import_path,
                            "import_transform_failed",
                            error,
                        ));
                    }
                }
            }

            let addressable_paths = match host.resolve_import(import_handle) {
                Ok(scene_import) => scene_import.addressable_node_paths(),
                Err(error) => {
                    diagnostics.push(scene_host_error_diagnostic(
                        &import_path,
                        "import_paths_failed",
                        error,
                    ));
                    BTreeMap::new()
                }
            };
            let nodes_by_path = addressable_paths
                .into_iter()
                .map(|(path, node)| (format!("{}:{path}", import.id), host.register_node(node)))
                .collect::<BTreeMap<_, _>>();

            imports.push(SceneRecipeBuildImportV1 {
                id: import.id.clone(),
                uri: resolved_uri,
                import_handle,
                primary_root: root_handles.first().copied(),
                root_handles,
                nodes_by_path,
            });
        }

        let import_handles = imports
            .iter()
            .map(|import| (import.id.clone(), import.import_handle))
            .collect::<BTreeMap<_, _>>();
        let authored_start = diagnostics.len();
        let geometry_handles = build_authored_geometries(
            &policy,
            &host,
            &recipe.colors,
            &recipe.geometries,
            &mut geometries,
            &mut diagnostics,
        );
        let material_handles = build_authored_materials(
            &policy,
            &host,
            recipe_path,
            &recipe.colors,
            &recipe.materials,
            &mut materials,
            &mut diagnostics,
        )
        .await;
        let node_keys = build_authored_nodes(
            &policy,
            &mut host,
            &recipe.nodes,
            &recipe.colors,
            AuthoredNodeResources {
                geometries: &geometry_handles,
                materials: &material_handles,
                imports: &import_handles,
            },
            &mut nodes,
            &mut diagnostics,
        );
        let instance_set_keys = build_authored_instance_sets(
            &policy,
            &mut host,
            &recipe.instance_sets,
            &recipe.colors,
            InstanceSetResources {
                geometries: &geometry_handles,
                materials: &material_handles,
                nodes: &node_keys,
                imports: &import_handles,
            },
            &mut nodes,
            &mut diagnostics,
        );
        let mut target_node_keys = node_keys.clone();
        target_node_keys.extend(instance_set_keys);
        let label_keys = build_authored_labels(
            &mut host,
            &recipe.labels,
            &recipe.colors,
            &target_node_keys,
            &import_handles,
            &mut nodes,
            &mut diagnostics,
        );
        target_node_keys.extend(label_keys);
        build_authored_clipping_planes(&mut host, &recipe.clipping_planes, &mut diagnostics);
        build_authored_cameras(
            &mut host,
            &recipe.cameras,
            &target_node_keys,
            &mut cameras,
            &mut diagnostics,
        );
        build_authored_lights(
            &mut host,
            &recipe.colors,
            &recipe.lights,
            &mut lights,
            &mut diagnostics,
        );
        apply_scene_setup(
            &policy,
            &mut host,
            recipe_path,
            &recipe.colors,
            recipe.scene.as_ref(),
            &mut diagnostics,
        )
        .await;
        apply_recipe_overlays(&mut host, &recipe, &imports, &nodes, &mut diagnostics);

        let mut manifest = build_manifest(diagnostics, skipped);
        manifest.imports = imports;
        manifest.geometries = geometries;
        manifest.materials = materials;
        manifest.nodes = nodes;
        manifest.cameras = cameras;
        manifest.lights = lights;
        if manifest.ok && !has_errors(&manifest.diagnostics[authored_start..]) {
            Ok(SceneHostRecipeBuild { host, manifest })
        } else {
            Err(manifest)
        }
    }
}

fn recipe_headless_host(
    width: u32,
    height: u32,
    options: crate::RendererOptions,
) -> Result<SceneHostCore<DefaultAssetFetcher>, SceneHostError> {
    let viewport = SurfaceViewport::new(width as f32, height as f32, 1.0).ok_or_else(|| {
        SceneHostError::new(
            SceneHostErrorCode::InvalidViewport,
            format!("invalid viewport {width}x{height} at DPR 1"),
        )
    })?;
    SceneHostCore::from_renderer(
        Assets::new(),
        Renderer::headless_with_options(width, height, options)?,
        viewport,
    )
}

fn build_manifest(
    diagnostics: Vec<SceneRecipeDiagnosticV1>,
    skipped: Vec<SceneRecipeBuildSkippedV1>,
) -> SceneRecipeBuildV1 {
    SceneRecipeBuildV1 {
        schema: SCENE_RECIPE_BUILD_SCHEMA_V1.to_owned(),
        ok: !has_errors(&diagnostics),
        imports: Vec::new(),
        nodes: Vec::<SceneRecipeBuildTargetV1>::new(),
        cameras: Vec::new(),
        lights: Vec::new(),
        geometries: Vec::<SceneRecipeBuildResourceV1>::new(),
        materials: Vec::new(),
        diagnostics,
        skipped,
    }
}

fn has_errors(diagnostics: &[SceneRecipeDiagnosticV1]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "error")
}

pub(super) fn scene_host_error_diagnostic(
    path: impl Into<String>,
    code: impl Into<String>,
    error: SceneHostError,
) -> SceneRecipeDiagnosticV1 {
    error_diagnostic(
        path,
        code,
        error.to_string(),
        "fix the recipe input and retry",
    )
}

pub(super) fn error_diagnostic(
    path: impl Into<String>,
    code: impl Into<String>,
    message: impl Into<String>,
    help: impl Into<String>,
) -> SceneRecipeDiagnosticV1 {
    build_diagnostic(code, "error", path, message, help, None, false)
}
