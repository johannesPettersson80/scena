use std::collections::BTreeMap;

use super::{SceneHostCore, SceneHostError};
use crate::AssetPath;
use crate::assets::{AssetLoadOptions, DefaultAssetFetcher};
use crate::diagnostics::AssetError;
use crate::scene::recipe::{
    RecipeBuildPolicy, RecipeBuildResultV1, SCENE_RECIPE_BUILD_SCHEMA_V1, SceneRecipeBuildImportV1,
    SceneRecipeBuildResourceV1, SceneRecipeBuildSkippedV1, SceneRecipeBuildTargetV1,
    SceneRecipeBuildV1, SceneRecipeDiagnosticV1, build_diagnostic,
    parse_valid_scene_recipe_json_with_policy,
};

mod authoring;
mod backend;
mod diagnostic;
mod host;
mod import_presentation;
mod manifest;
mod overlays;
mod policy;
mod setup;
mod spatial_state;
mod types;

use authoring::{
    AuthoredMaterialResources, AuthoredNodeResources, InstanceSetResources, LabelResources,
    ParticleSetResources, apply_import_transform, build_authored_animations,
    build_authored_cameras, build_authored_clipping_planes, build_authored_fonts,
    build_authored_geometries, build_authored_instance_sets, build_authored_labels,
    build_authored_lights, build_authored_materials, build_authored_morphs, build_authored_nodes,
    build_authored_particle_sets, build_authored_skins,
};
pub(super) use diagnostic::{error_diagnostic, scene_host_error_diagnostic};
use host::{RecipeBackendPolicy, recipe_headless_host, recipe_manifest_host};
use import_presentation::apply_import_presentation;
use manifest::{RecipeBuildFailure, build_manifest, has_errors};
use overlays::apply_recipe_overlays;
use policy::{
    RecipeBuildBudget, RecipeTextureBudget, asset_policy_diagnostics, recipe_policy_diagnostics,
};
use setup::{
    apply_render_setup, apply_scene_setup, renderer_options_from_recipe,
    validate_scene_setup_for_manifest,
};
use spatial_state::{SpatialBuildInputs, SpatialStateManifest, build_spatial_state};
use types::RecipeBuildMode;
pub use types::SceneHostRecipeBuild;

impl SceneHostCore<DefaultAssetFetcher> {
    async fn build_recipe_json_with_backend(
        recipe_path: impl AsRef<str>,
        text: &str,
        policy: RecipeBuildPolicy,
        backend_policy: RecipeBackendPolicy,
    ) -> Result<SceneHostRecipeBuild<DefaultAssetFetcher>, SceneRecipeBuildV1> {
        Self::build_recipe_json_with_mode(
            recipe_path,
            text,
            policy,
            RecipeBuildMode::Host(backend_policy),
        )
        .await
        .map_err(|failure| failure.manifest)
    }

    async fn build_recipe_json_with_mode(
        recipe_path: impl AsRef<str>,
        text: &str,
        policy: RecipeBuildPolicy,
        mode: RecipeBuildMode,
    ) -> Result<SceneHostRecipeBuild<DefaultAssetFetcher>, RecipeBuildFailure> {
        let recipe_path = recipe_path.as_ref();
        let recipe = match parse_valid_scene_recipe_json_with_policy(text, &policy) {
            Ok(recipe) => recipe,
            Err(report) => {
                return Err(RecipeBuildFailure::before_host(build_manifest(
                    report.diagnostics,
                    Vec::new(),
                )));
            }
        };

        let mut diagnostics = recipe_policy_diagnostics(&recipe, &policy);
        let mut skipped = Vec::new();
        let resource_plan: crate::scene::recipe::RecipeResourcePlan =
            policy.resolve_recipe_resources(recipe_path, &recipe);
        diagnostics.extend(resource_plan.diagnostics.iter().cloned());
        if has_errors(&diagnostics) {
            return Err(RecipeBuildFailure::before_host(build_manifest(
                diagnostics,
                skipped,
            )));
        }

        let (width, height) = recipe
            .capture
            .as_ref()
            .map(|capture| (capture.width, capture.height))
            .unwrap_or((800, 600));
        let host_result = match mode {
            RecipeBuildMode::Host(backend_policy) => recipe_headless_host(
                width,
                height,
                renderer_options_from_recipe(recipe.render.as_ref()),
                backend_policy,
            ),
            RecipeBuildMode::ManifestOnly => recipe_manifest_host(width, height),
        };
        let mut host = match host_result {
            Ok(host) => host,
            Err(error) => {
                diagnostics.push(scene_host_error_diagnostic("$", "build_failed", error));
                return Err(RecipeBuildFailure::before_host(build_manifest(
                    diagnostics,
                    skipped,
                )));
            }
        };
        if matches!(mode, RecipeBuildMode::Host(_)) {
            apply_render_setup(&mut host, recipe.render.as_ref(), &mut diagnostics);
        }
        if has_errors(&diagnostics) {
            return Err(RecipeBuildFailure::with_host(
                build_manifest(diagnostics, skipped),
                &host,
            ));
        }
        let mut imports = Vec::new();
        let mut geometries = Vec::new();
        let mut materials = Vec::new();
        let mut fonts = Vec::new();
        let mut nodes = Vec::new();
        let mut instances = Vec::new();
        let mut cameras = Vec::new();
        let mut lights = Vec::new();
        let mut animations = Vec::new();
        let mut anchors = Vec::new();
        let mut connectors = Vec::new();
        let mut connections = Vec::new();
        let mut bounds = Vec::new();
        let mut named_states = Vec::new();
        let mut imported_node_keys = BTreeMap::new();
        let mut build_budget = RecipeBuildBudget::default();
        let mut texture_budget = RecipeTextureBudget::default();

        for (index, import) in recipe.imports.iter().enumerate() {
            let import_path = format!("$.imports[{index}]");
            let resource_path = format!("{import_path}.uri");
            let Some(resolved_uri) = resource_plan
                .resolved_uri(&resource_path)
                .map(str::to_owned)
            else {
                continue;
            };

            let report =
                match host
                    .assets
                    .load_scene_with_report_options(
                        AssetPath::from(resolved_uri.as_str()),
                        AssetLoadOptions::default()
                            .with_fetch_byte_limit(policy.fetch_byte_limit()),
                    )
                    .await
                {
                    Err(AssetError::PolicyViolation { reason, help, .. }) => {
                        diagnostics.push(error_diagnostic(
                            &import_path,
                            "policy_violation",
                            reason,
                            help,
                        ));
                        continue;
                    }
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
                &mut build_budget,
                &mut texture_budget,
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
            apply_import_transform(
                &mut host,
                &root_handles,
                import.transform.as_ref(),
                &import_path,
                &mut diagnostics,
            );
            apply_import_presentation(
                &mut host,
                &recipe.colors,
                import,
                &root_handles,
                &import_path,
                &mut nodes,
                &mut diagnostics,
            );
            if has_errors(&diagnostics[diagnostic_start..]) {
                continue;
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
                .map(|(path, node)| {
                    let id = format!("{}:{path}", import.id);
                    imported_node_keys.insert(id.clone(), node);
                    (id, host.register_node(node))
                })
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
        let mut geometry_handles = build_authored_geometries(
            &policy,
            &host,
            &recipe.colors,
            &recipe.geometries,
            &mut build_budget,
            &mut geometries,
            &mut diagnostics,
        );
        let morph_handles = build_authored_morphs(
            &policy,
            &host,
            &recipe.morphs,
            &geometry_handles,
            &mut build_budget,
            &mut geometries,
            &mut diagnostics,
        );
        geometry_handles.extend(morph_handles);
        let skin_handles = build_authored_skins(
            &policy,
            &host,
            &recipe.skins,
            &geometry_handles,
            &mut build_budget,
            &mut geometries,
            &mut diagnostics,
        );
        geometry_handles.extend(skin_handles);
        let material_handles = build_authored_materials(
            &policy,
            &host,
            recipe_path,
            &recipe.materials,
            AuthoredMaterialResources {
                colors: &recipe.colors,
                build_budget: &mut build_budget,
                texture_budget: &mut texture_budget,
            },
            &mut materials,
            &mut diagnostics,
        )
        .await;
        let font_faces = build_authored_fonts(
            &policy,
            recipe_path,
            &recipe.fonts,
            &mut fonts,
            &mut skipped,
            &mut diagnostics,
        );
        let node_keys = build_authored_nodes(
            &policy,
            &mut host,
            &recipe.nodes,
            AuthoredNodeResources {
                colors: &recipe.colors,
                geometries: &geometry_handles,
                materials: &material_handles,
                imported_nodes: &imported_node_keys,
                imports: &import_handles,
                build_budget: &mut build_budget,
            },
            &mut nodes,
            &mut diagnostics,
        );
        let instance_set_keys = build_authored_instance_sets(
            &policy,
            &mut host,
            &recipe.instance_sets,
            InstanceSetResources {
                colors: &recipe.colors,
                geometries: &geometry_handles,
                materials: &material_handles,
                nodes: &node_keys,
                imported_nodes: &imported_node_keys,
                imports: &import_handles,
                build_budget: &mut build_budget,
            },
            &mut nodes,
            &mut instances,
            &mut diagnostics,
        );
        let mut target_node_keys = node_keys.clone();
        target_node_keys.extend(imported_node_keys.clone());
        target_node_keys.extend(instance_set_keys);
        let particle_set_keys = build_authored_particle_sets(
            &policy,
            &mut host,
            &recipe.particles,
            &recipe.colors,
            ParticleSetResources {
                nodes: &target_node_keys,
                imports: &import_handles,
            },
            &mut nodes,
            &mut diagnostics,
        );
        target_node_keys.extend(particle_set_keys);
        let label_keys = build_authored_labels(
            &mut host,
            &recipe.labels,
            LabelResources {
                colors: &recipe.colors,
                fonts: &font_faces,
                nodes: &target_node_keys,
                imports: &import_handles,
            },
            &mut nodes,
            &mut diagnostics,
        );
        target_node_keys.extend(label_keys);
        build_spatial_state(
            &mut host,
            &recipe.colors,
            &recipe.anchors,
            &recipe.connectors,
            &recipe.bounds,
            &recipe.named_states,
            SpatialBuildInputs {
                node_keys: &target_node_keys,
                imported_node_keys: &imported_node_keys,
                import_handles: &import_handles,
                imports: &imports,
            },
            SpatialStateManifest {
                anchors: &mut anchors,
                connectors: &mut connectors,
                connections: &mut connections,
                bounds: &mut bounds,
                named_states: &mut named_states,
            },
            &mut diagnostics,
        );
        build_authored_clipping_planes(&mut host, &recipe.clipping_planes, &mut diagnostics);
        build_authored_animations(
            &policy,
            &mut host,
            &recipe.animations,
            &target_node_keys,
            &mut build_budget,
            &mut animations,
            &mut diagnostics,
        );
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
        match mode {
            RecipeBuildMode::Host(_) => {
                apply_scene_setup(
                    &policy,
                    &mut host,
                    recipe_path,
                    &recipe.colors,
                    recipe.scene.as_ref(),
                    &mut texture_budget,
                    &mut diagnostics,
                )
                .await
            }
            RecipeBuildMode::ManifestOnly => {
                validate_scene_setup_for_manifest(
                    &policy,
                    &mut host,
                    recipe_path,
                    &recipe.colors,
                    recipe.scene.as_ref(),
                    &mut texture_budget,
                    &mut diagnostics,
                )
                .await
            }
        }
        apply_recipe_overlays(&mut host, &recipe, &imports, &nodes, &mut diagnostics);

        let mut manifest = build_manifest(diagnostics, skipped);
        manifest.imports = imports;
        manifest.geometries = geometries;
        manifest.materials = materials;
        manifest.fonts = fonts;
        manifest.nodes = nodes;
        manifest.instances = instances;
        manifest.cameras = cameras;
        manifest.lights = lights;
        manifest.animations = animations;
        manifest.anchors = anchors;
        manifest.connectors = connectors;
        manifest.connections = connections;
        manifest.bounds = bounds;
        manifest.named_states = named_states;
        if manifest.ok && !has_errors(&manifest.diagnostics[authored_start..]) {
            Ok(SceneHostRecipeBuild { host, manifest })
        } else {
            Err(RecipeBuildFailure::with_host(manifest, &host))
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::Renderer;

    #[test]
    fn recipe_instance_sets_change_headless_gpu_pixels_by_transform_tint_and_visibility() {
        let recipe = serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "white": "#FFFFFF",
                "red": "#E03030",
                "blue": "#2050E0",
                "green": "#20D060"
            },
            "geometries": [
                { "id": "quad_geo", "primitive": { "kind": "box", "size": [0.18, 0.18, 0.02] } }
            ],
            "materials": [
                { "id": "quad_mat", "kind": "unlit", "base_color": "white", "double_sided": true }
            ],
            "instance_sets": [{
                "id": "instanced_pixels",
                "geometry": "quad_geo",
                "material": "quad_mat",
                "instances": [
                    {
                        "id": "left_red",
                        "transform": { "kind": "trs", "translation": [-0.32, 0.0, 0.0] },
                        "tint": "red"
                    },
                    {
                        "id": "right_blue",
                        "transform": { "kind": "trs", "translation": [0.32, 0.0, 0.0] },
                        "tint": "blue"
                    },
                    {
                        "id": "hidden_green",
                        "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.0] },
                        "tint": "green",
                        "visible": false
                    }
                ]
            }],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.7], "target": [0.0, 0.0, 0.0] }
            }],
            "capture": { "width": 160, "height": 120 }
        }))
        .expect("recipe serializes");

        let build = pollster::block_on(SceneHostCore::build_recipe_json(
            "tests/assets/slice6-instance-pixels.recipe.json",
            &recipe,
            RecipeBuildPolicy::testing(),
        ))
        .expect("recipe builds");
        assert!(build.manifest.ok, "{:#?}", build.manifest);

        let mut scene = build.host.scene;
        let assets = build.host.assets;
        let camera = build.host.active_camera;
        let mut renderer = Renderer::headless_gpu(160, 120).expect("HeadlessGpu renderer builds");
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("instance recipe prepares on HeadlessGpu");
        renderer
            .render(&scene, camera)
            .expect("instance recipe renders on HeadlessGpu");
        let rgba = renderer.frame_rgba8();

        let red = color_bounds(rgba, 160, |pixel| {
            pixel[0] > 150 && pixel[1] < 100 && pixel[2] < 100
        })
        .expect("red instance pixels are visible");
        let blue = color_bounds(rgba, 160, |pixel| {
            pixel[2] > 130 && pixel[0] < 100 && pixel[1] < 120
        })
        .expect("blue instance pixels are visible");
        let green = color_bounds(rgba, 160, |pixel| {
            pixel[1] > 130 && pixel[0] < 100 && pixel[2] < 120
        });

        assert!(
            red.center_x() < 70.0 && blue.center_x() > 90.0,
            "per-instance transforms should move rendered pixels: red={red:?}, blue={blue:?}"
        );
        assert!(
            (red.center_y() - 60.0).abs() < 8.0 && (blue.center_y() - 60.0).abs() < 8.0,
            "instance y positions should remain centered: red={red:?}, blue={blue:?}"
        );
        assert!(
            green.is_none(),
            "hidden instance must not produce green pixels: {green:?}"
        );
    }

    #[test]
    fn recipe_primitives_render_lit_single_sided_pixels_on_headless_gpu() {
        let cases = [
            (
                "box",
                json!({ "kind": "box", "size": [0.20, 0.12, 0.16] }),
                json!({ "kind": "trs" }),
            ),
            (
                "sphere",
                json!({ "kind": "sphere", "radius": 0.11, "segments": 12, "rings": 6 }),
                json!({ "kind": "trs" }),
            ),
            (
                "cylinder",
                json!({ "kind": "cylinder", "radius": 0.10, "height": 0.22, "segments": 12 }),
                json!({ "kind": "trs" }),
            ),
            (
                "plane",
                json!({ "kind": "plane", "size": [0.20, 0.16] }),
                json!({ "kind": "trs", "rotation_degrees": [70.0, 0.0, 0.0] }),
            ),
            (
                "cone",
                json!({ "kind": "cone", "radius": 0.10, "height": 0.22, "segments": 12 }),
                json!({ "kind": "trs" }),
            ),
            (
                "torus",
                json!({ "kind": "torus", "major_radius": 0.11, "minor_radius": 0.03, "segments": 12, "rings": 6 }),
                json!({ "kind": "trs", "rotation_degrees": [65.0, 0.0, 0.0] }),
            ),
            (
                "disc",
                json!({ "kind": "disc", "radius": 0.12, "segments": 16 }),
                json!({ "kind": "trs", "rotation_degrees": [70.0, 0.0, 0.0] }),
            ),
            (
                "wedge",
                json!({ "kind": "wedge", "size": [0.20, 0.12, 0.16] }),
                json!({ "kind": "trs" }),
            ),
        ];

        for (name, primitive, transform) in cases {
            let recipe = serde_json::to_string_pretty(&json!({
                "schema": "scena.scene_recipe.v1",
                "colors": {
                    "mat_color": "#E8C060"
                },
                "geometries": [
                    { "id": "geo", "primitive": primitive }
                ],
                "materials": [
                    { "id": "mat", "kind": "pbr_metallic_roughness", "base_color": "mat_color", "metallic": 0.0, "roughness": 0.55 }
                ],
                "lights": [{
                    "id": "key",
                    "kind": "directional",
                    "preset": "key",
                    "illuminance_lux": 9000.0
                }],
                "nodes": [
                    { "id": "node", "geometry": "geo", "material": "mat", "transform": transform }
                ],
                "cameras": [{
                    "id": "main",
                    "kind": "perspective",
                    "active": true,
                    "transform": { "kind": "look_at", "eye": [0.0, 0.42, 0.72], "target": [0.0, 0.0, 0.0] }
                }],
                "capture": { "width": 96, "height": 72 }
            }))
            .expect("recipe serializes");

            let build = pollster::block_on(SceneHostCore::build_recipe_json(
                "tests/assets/slice10-primitive-pixels.recipe.json",
                &recipe,
                RecipeBuildPolicy::testing(),
            ))
            .unwrap_or_else(|error| panic!("{name} recipe builds: {error:#?}"));
            assert!(build.manifest.ok, "{name}: {:#?}", build.manifest);

            let mut scene = build.host.scene;
            let assets = build.host.assets;
            let camera = build.host.active_camera;
            let mut renderer = Renderer::headless_gpu(96, 72).expect("HeadlessGpu renderer builds");
            renderer.set_background_color(crate::Color::from_srgb_u8(18, 24, 32));
            renderer
                .prepare_with_assets(&mut scene, &assets)
                .unwrap_or_else(|error| panic!("{name} prepares on HeadlessGpu: {error:#?}"));
            renderer
                .render(&scene, camera)
                .unwrap_or_else(|error| panic!("{name} renders on HeadlessGpu: {error:#?}"));
            let rgba = renderer.frame_rgba8();
            let bounds = color_bounds(rgba, 96, |pixel| {
                let r = i16::from(pixel[0]);
                let g = i16::from(pixel[1]);
                let b = i16::from(pixel[2]);
                (r - 18).abs() > 10 || (g - 24).abs() > 10 || (b - 32).abs() > 10
            })
            .unwrap_or_else(|| panic!("{name} must render visible non-background pixels"));

            assert!(
                bounds.pixel_count() > 12,
                "{name} should have a measurable per-primitive silhouette: {bounds:?}"
            );
            assert!(
                (bounds.center_x() - 48.0).abs() < 20.0 && (bounds.center_y() - 36.0).abs() < 20.0,
                "{name} silhouette should be framed near the capture center: {bounds:?}"
            );
        }
    }

    #[test]
    fn recipe_environment_changes_lit_pbr_pixels_on_headless_gpu() {
        let without_environment = render_recipe_environment_gpu(false);
        let with_environment = render_recipe_environment_gpu(true);
        let delta = frame_abs_diff(&without_environment, &with_environment);
        assert!(
            delta > 100,
            "recipe scene.environment must alter lit PBR pixels on HeadlessGpu, delta={delta}"
        );
    }

    fn render_recipe_environment_gpu(enable_environment: bool) -> Vec<u8> {
        let environment = if enable_environment {
            json!({ "kind": "default" })
        } else {
            json!({ "kind": "none" })
        };
        let recipe = serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "mat_color": "#D8A64A"
            },
            "geometries": [
                { "id": "body_geo", "primitive": { "kind": "sphere", "radius": 0.22, "segments": 24, "rings": 12 } }
            ],
            "materials": [
                { "id": "body_mat", "kind": "pbr_metallic_roughness", "base_color": "mat_color", "metallic": 0.0, "roughness": 0.38 }
            ],
            "nodes": [
                { "id": "body", "geometry": "body_geo", "material": "body_mat" }
            ],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.1], "target": "body" }
            }],
            "scene": {
                "background": { "kind": "black" },
                "environment": environment
            },
            "capture": { "width": 96, "height": 72 }
        }))
        .expect("recipe serializes");

        let build = pollster::block_on(SceneHostCore::build_recipe_json(
            "tests/assets/slice4-environment-pixels.recipe.json",
            &recipe,
            RecipeBuildPolicy::testing(),
        ))
        .expect("environment recipe builds");
        assert!(build.manifest.ok, "{:#?}", build.manifest);
        let environment = build.host.renderer.environment();
        let mut scene = build.host.scene;
        let assets = build.host.assets;
        let camera = build.host.active_camera;
        let mut renderer = Renderer::headless_gpu(96, 72).expect("HeadlessGpu renderer builds");
        renderer.set_background_color(crate::Color::BLACK);
        if let Some(environment) = environment {
            renderer.set_environment(environment);
        }
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("environment recipe prepares on HeadlessGpu");
        renderer
            .render(&scene, camera)
            .expect("environment recipe renders on HeadlessGpu");
        renderer.frame_rgba8().to_vec()
    }

    fn frame_abs_diff(before: &[u8], after: &[u8]) -> u64 {
        assert_eq!(before.len(), after.len(), "frames must match");
        before
            .iter()
            .zip(after)
            .map(|(before, after)| u64::from(before.abs_diff(*after)))
            .sum()
    }

    #[derive(Debug)]
    struct ColorBounds {
        min_x: usize,
        min_y: usize,
        max_x: usize,
        max_y: usize,
    }

    impl ColorBounds {
        fn center_x(&self) -> f32 {
            (self.min_x + self.max_x) as f32 * 0.5
        }

        fn center_y(&self) -> f32 {
            (self.min_y + self.max_y) as f32 * 0.5
        }

        fn pixel_count(&self) -> usize {
            (self.max_x - self.min_x + 1) * (self.max_y - self.min_y + 1)
        }
    }

    fn color_bounds(
        rgba: &[u8],
        width: usize,
        matches: impl Fn(&[u8]) -> bool,
    ) -> Option<ColorBounds> {
        let mut bounds: Option<ColorBounds> = None;
        for (index, pixel) in rgba.chunks_exact(4).enumerate() {
            if !matches(pixel) {
                continue;
            }
            let x = index % width;
            let y = index / width;
            bounds = Some(match bounds {
                Some(mut bounds) => {
                    bounds.min_x = bounds.min_x.min(x);
                    bounds.min_y = bounds.min_y.min(y);
                    bounds.max_x = bounds.max_x.max(x);
                    bounds.max_y = bounds.max_y.max(y);
                    bounds
                }
                None => ColorBounds {
                    min_x: x,
                    min_y: y,
                    max_x: x,
                    max_y: y,
                },
            });
        }
        bounds
    }
}
