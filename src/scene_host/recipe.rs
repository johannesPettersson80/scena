use std::collections::BTreeMap;

use super::{SceneHostCore, SceneHostError};
use crate::assets::{AssetLoadReport, DefaultAssetFetcher, SceneAsset, TextureHandle};
use crate::material::MaterialDesc;
use crate::scene::recipe::{
    RecipeBuildPolicy, SCENE_RECIPE_BUILD_SCHEMA_V1, SceneRecipeBuildImportV1,
    SceneRecipeBuildResourceV1, SceneRecipeBuildSkippedV1, SceneRecipeBuildTargetV1,
    SceneRecipeBuildV1, SceneRecipeDiagnosticV1, build_diagnostic, parse_valid_scene_recipe_json,
};
use crate::{AssetPath, GeometryHandle, MaterialHandle};

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
        let mut host = match Self::headless(width, height) {
            Ok(host) => host,
            Err(error) => {
                diagnostics.push(scene_host_error_diagnostic("$", "build_failed", error));
                return Err(build_manifest(diagnostics, skipped));
            }
        };
        let mut imports = Vec::new();

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

        let mut manifest = build_manifest(diagnostics, skipped);
        manifest.imports = imports;
        if manifest.ok {
            Ok(SceneHostRecipeBuild { host, manifest })
        } else {
            Err(manifest)
        }
    }
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

fn asset_policy_diagnostics(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    report: &AssetLoadReport<SceneAsset>,
    import_path: &str,
) -> Vec<SceneRecipeDiagnosticV1> {
    let mut diagnostics = Vec::new();
    let fetched_bytes = report.fetched_bytes()
        + report
            .external_resources()
            .iter()
            .filter_map(|resource| resource.bytes)
            .sum::<usize>();
    if fetched_bytes > policy.fetch_byte_limit() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import fetched {fetched_bytes} bytes, exceeding RecipeBuildPolicy fetch_byte_limit {}",
                policy.fetch_byte_limit()
            ),
            "use a smaller asset or raise the operator-owned fetch_byte_limit policy",
        ));
    }

    let asset = report.asset();
    let node_count = asset.node_count();
    if node_count > policy.max_nodes() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import contains {node_count} nodes, exceeding RecipeBuildPolicy max_nodes {}",
                policy.max_nodes()
            ),
            "use a smaller asset or raise the operator-owned max_nodes policy",
        ));
    }

    let instance_count = asset
        .nodes()
        .iter()
        .map(|node| node.instance_transforms().len())
        .sum::<usize>();
    if instance_count > policy.max_instances() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import contains {instance_count} authored instances, exceeding RecipeBuildPolicy max_instances {}",
                policy.max_instances()
            ),
            "use fewer instances or raise the operator-owned max_instances policy",
        ));
    }

    let mut geometries = Vec::<GeometryHandle>::new();
    let mut materials = Vec::<MaterialHandle>::new();
    for node in asset.nodes() {
        for mesh in node.meshes() {
            push_unique(&mut geometries, mesh.geometry());
            push_unique(&mut materials, mesh.material());
            for binding in mesh.material_variant_bindings() {
                push_unique(&mut materials, binding.material());
            }
        }
    }

    if materials.len() > policy.max_materials() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import references {} materials, exceeding RecipeBuildPolicy max_materials {}",
                materials.len(),
                policy.max_materials()
            ),
            "use fewer materials or raise the operator-owned max_materials policy",
        ));
    }

    let mut vertex_count = 0usize;
    let mut index_count = 0usize;
    for geometry in geometries {
        if let Some(desc) = host.assets.geometry(geometry) {
            vertex_count = vertex_count.saturating_add(desc.vertices().len());
            index_count = index_count.saturating_add(desc.indices().len());
        }
    }
    if vertex_count > policy.max_vertices() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import references {vertex_count} vertices, exceeding RecipeBuildPolicy max_vertices {}",
                policy.max_vertices()
            ),
            "use lower-detail geometry or raise the operator-owned max_vertices policy",
        ));
    }
    if index_count > policy.max_indices() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import references {index_count} indices, exceeding RecipeBuildPolicy max_indices {}",
                policy.max_indices()
            ),
            "use lower-detail geometry or raise the operator-owned max_indices policy",
        ));
    }

    let mut textures = Vec::<TextureHandle>::new();
    for material in materials {
        if let Some(desc) = host.assets.material(material) {
            collect_material_textures(&desc, &mut textures);
        }
    }
    if textures.len() > policy.max_textures() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "import references {} textures, exceeding RecipeBuildPolicy max_textures {}",
                textures.len(),
                policy.max_textures()
            ),
            "use fewer textures or raise the operator-owned max_textures policy",
        ));
    }
    let mut decoded_texture_bytes = 0usize;
    for texture in textures {
        let Some(desc) = host.assets.texture(texture) else {
            continue;
        };
        if let Some((width, height, rgba8)) = desc.decoded_rgba8() {
            decoded_texture_bytes = decoded_texture_bytes.saturating_add(rgba8.len());
            let max_dimension = width.max(height);
            if max_dimension > policy.max_image_dimension() {
                diagnostics.push(error_diagnostic(
                    import_path,
                    "policy_violation",
                    format!(
                        "texture dimensions {width}x{height} exceed RecipeBuildPolicy max_image_dimension {}",
                        policy.max_image_dimension()
                    ),
                    "use smaller textures or raise the operator-owned max_image_dimension policy",
                ));
            }
        } else if let Some((width, height)) = desc.decoded_dimensions() {
            let max_dimension = width.max(height);
            if max_dimension > policy.max_image_dimension() {
                diagnostics.push(error_diagnostic(
                    import_path,
                    "policy_violation",
                    format!(
                        "texture dimensions {width}x{height} exceed RecipeBuildPolicy max_image_dimension {}",
                        policy.max_image_dimension()
                    ),
                    "use smaller textures or raise the operator-owned max_image_dimension policy",
                ));
            }
        }
    }
    if decoded_texture_bytes > policy.max_texture_bytes() {
        diagnostics.push(error_diagnostic(
            import_path,
            "policy_violation",
            format!(
                "decoded textures use {decoded_texture_bytes} bytes, exceeding RecipeBuildPolicy max_texture_bytes {}",
                policy.max_texture_bytes()
            ),
            "use smaller textures or raise the operator-owned max_texture_bytes policy",
        ));
    }

    diagnostics
}

fn collect_material_textures(material: &MaterialDesc, textures: &mut Vec<TextureHandle>) {
    for texture in [
        material.base_color_texture(),
        material.normal_texture(),
        material.metallic_roughness_texture(),
        material.occlusion_texture(),
        material.emissive_texture(),
        material.clearcoat_texture(),
        material.clearcoat_roughness_texture(),
        material.clearcoat_normal_texture(),
        material.sheen_color_texture(),
        material.sheen_roughness_texture(),
        material.anisotropy_texture(),
        material.iridescence_texture(),
        material.iridescence_thickness_texture(),
        material.transmission_texture(),
        material.thickness_texture(),
    ]
    .into_iter()
    .flatten()
    {
        push_unique(textures, texture);
    }
}

fn push_unique<T: Copy + PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn scene_host_error_diagnostic(
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

fn error_diagnostic(
    path: impl Into<String>,
    code: impl Into<String>,
    message: impl Into<String>,
    help: impl Into<String>,
) -> SceneRecipeDiagnosticV1 {
    build_diagnostic(code, "error", path, message, help, None, false)
}
