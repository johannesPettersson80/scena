use std::collections::{BTreeMap, BTreeSet};

use super::{SceneHostCore, error_diagnostic, has_errors, scene_host_error_diagnostic};
use crate::assets::{AssetPath, DefaultAssetFetcher, MaterialImperfectionDesc};
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeBuildTargetV1, SceneRecipeColorV1, SceneRecipeDiagnosticV1,
    SceneRecipeImportEdgeRoundingReportV1, SceneRecipeImportMaterialV1, SceneRecipeImportV1,
};
use crate::scene_host::recipe::authoring::{DiagnosticPathExt, authored_color};
use crate::scene_host::recipe::policy::RecipeTextureBudget;
use crate::{
    Color, MaterialDesc, MaterialHandle, NodeKey,
    geometry::edge_rounding::{EdgeRoundingError, EdgeRoundingOptions, round_hard_edges},
};

struct ResolvedImportMaterialBinding {
    source_index: usize,
    source_name: Option<String>,
    material: MaterialHandle,
    path: String,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn apply_import_presentation(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    import: &SceneRecipeImportV1,
    root_handles: &[u64],
    import_path: &str,
    texture_budget: &mut RecipeTextureBudget,
    generated_nodes: &mut Vec<SceneRecipeBuildTargetV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<SceneRecipeImportEdgeRoundingReportV1> {
    let edge_rounding_report =
        apply_import_edge_rounding(host, import, root_handles, import_path, diagnostics);
    let material_handle = match import.material.as_ref() {
        Some(material) => {
            import_material_handle(
                policy,
                host,
                recipe_path,
                colors,
                material,
                &format!("{import_path}.material"),
                texture_budget,
                diagnostics,
            )
            .await
        }
        None => None,
    };
    let material_bindings = import_material_binding_handles(
        policy,
        host,
        recipe_path,
        colors,
        import,
        import_path,
        texture_budget,
        diagnostics,
    )
    .await;
    let edge_material_handle =
        import_edge_material_handle(host, colors, import, import_path, diagnostics);
    if has_errors(diagnostics) {
        return edge_rounding_report;
    }
    apply_import_material_bindings(host, root_handles, &material_bindings, diagnostics);
    if has_errors(diagnostics) {
        return edge_rounding_report;
    }
    for (root_index, root_handle) in root_handles.iter().enumerate() {
        let root = match host.resolve_node(*root_handle) {
            Ok(root) => root,
            Err(error) => {
                diagnostics.push(scene_host_error_diagnostic(
                    import_path,
                    "import_presentation_failed",
                    error,
                ));
                continue;
            }
        };
        if let Some(material) = material_handle
            && let Err(error) = host.scene.set_subtree_mesh_material(root, material)
        {
            diagnostics.push(error_diagnostic(
                import_path,
                "import_material_failed",
                format!("failed to apply import material override: {error}"),
                "check that the import root still resolves to imported mesh nodes",
            ));
        }
        let edge_nodes = match edge_material_handle {
            Some(edge_material) => {
                match host.scene.add_subtree_edge_overlays(root, edge_material) {
                    Ok(nodes) => nodes,
                    Err(error) => {
                        diagnostics.push(error_diagnostic(
                            import_path,
                            "import_edge_emphasis_failed",
                            format!("failed to add import edge overlays: {error}"),
                            "check that the import root still resolves to imported mesh nodes",
                        ));
                        Vec::new()
                    }
                }
            }
            None => Vec::new(),
        };
        for (overlay_index, overlay_node) in edge_nodes.into_iter().enumerate() {
            let handle = host.register_node(overlay_node);
            generated_nodes.push(SceneRecipeBuildTargetV1 {
                id: format!("{}.edge_emphasis.{root_index}.{overlay_index}", import.id),
                handle,
                kind: "generated_overlay".to_owned(),
                parent: Some(*root_handle),
                name: None,
                visible: Some(true),
                active: None,
            });
        }
    }
    edge_rounding_report
}

fn apply_import_edge_rounding(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    import: &SceneRecipeImportV1,
    root_handles: &[u64],
    import_path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<SceneRecipeImportEdgeRoundingReportV1> {
    let edge_rounding = import.edge_rounding.as_ref()?;
    let mut report = SceneRecipeImportEdgeRoundingReportV1 {
        enabled: edge_rounding.enabled,
        inspected_meshes: 0,
        rounded_meshes: 0,
        skipped_meshes: 0,
        eligible_edges: 0,
        rounded_edges: 0,
        skipped_edges: 0,
        rejected_edges: 0,
        removed_degenerate_triangles: 0,
        source_triangles: 0,
        derived_triangles: 0,
    };
    if !edge_rounding.enabled {
        return Some(report);
    }

    let mut roots = Vec::with_capacity(root_handles.len());
    for root_handle in root_handles {
        match host.resolve_node(*root_handle) {
            Ok(root) => roots.push(root),
            Err(error) => {
                diagnostics.push(scene_host_error_diagnostic(
                    format!("{import_path}.edge_rounding"),
                    "import_edge_rounding_failed",
                    error,
                ));
                return Some(report);
            }
        }
    }
    let mut subject_bounds = None;
    for root in &roots {
        match host.scene.node_world_bounds(*root, &host.assets) {
            Ok(Some(bounds)) => {
                subject_bounds = Some(
                    subject_bounds.map_or(bounds, |current: crate::Aabb| current.union(bounds)),
                );
            }
            Ok(None) => {}
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    format!("{import_path}.edge_rounding"),
                    "import_edge_rounding_bounds_failed",
                    format!("failed to resolve imported subject bounds: {error}"),
                    "ensure the imported mesh has finite geometry and transforms",
                ));
                return Some(report);
            }
        }
    }
    let Some(subject_bounds) = subject_bounds else {
        diagnostics.push(error_diagnostic(
            format!("{import_path}.edge_rounding"),
            "import_edge_rounding_bounds_missing",
            "edge rounding requires a bounded imported mesh",
            "supply finite static triangle geometry",
        ));
        return Some(report);
    };
    let subject_dominant_extent = (subject_bounds.half_extent() * 2.0)
        .max_element()
        .max(1.0e-6);

    let mut subtree = Vec::new();
    for root in roots {
        match host.scene.subtree_nodes(root) {
            Ok(nodes) => {
                for node in nodes {
                    if !subtree.contains(&node) {
                        subtree.push(node);
                    }
                }
            }
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    format!("{import_path}.edge_rounding"),
                    "import_edge_rounding_failed",
                    format!("failed to inspect imported subtree: {error}"),
                    "ensure the import roots remain present during recipe construction",
                ));
                return Some(report);
            }
        }
    }
    let inspection = host.scene.inspect_with_assets(&host.assets);
    let meshes = inspection
        .nodes()
        .iter()
        .filter(|node| subtree.contains(&node.node()))
        .filter_map(|node| Some((node.node(), node.mesh_geometry()?, node.world_transform())))
        .collect::<Vec<_>>();

    let mut remaining_triangles = edge_rounding.max_derived_triangles;
    for (node, geometry_handle, world_transform) in meshes {
        report.inspected_meshes += 1;
        let geometry = match host.assets.try_geometry(geometry_handle) {
            Ok(geometry) => geometry,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    format!("{import_path}.edge_rounding"),
                    "import_edge_rounding_geometry_missing",
                    format!("imported mesh geometry could not be resolved: {error}"),
                    "reload the source asset and retry recipe construction",
                ));
                report.rejected_edges += 1;
                continue;
            }
        };
        let local_scale = world_transform.scale.abs().max_element().max(1.0e-6);
        let radius = subject_dominant_extent * edge_rounding.radius_fraction as f32 / local_scale;
        let options = EdgeRoundingOptions::new(radius)
            .with_segments(edge_rounding.segments)
            .with_edge_angle_degrees(edge_rounding.edge_angle_threshold_degrees as f32)
            .with_curve_target_error(subject_dominant_extent / local_scale / 4096.0)
            .with_max_derived_triangles(remaining_triangles);
        match round_hard_edges(&geometry, options) {
            Ok((rounded, mesh_report)) => {
                let geometry_changed = rounded != geometry;
                report.eligible_edges += mesh_report.eligible_edges;
                report.rounded_edges += mesh_report.rounded_edges;
                report.skipped_edges += mesh_report.skipped_edges;
                report.rejected_edges += mesh_report.rejected_edges;
                report.removed_degenerate_triangles += mesh_report.removed_degenerate_triangles;
                report.source_triangles += mesh_report.source_triangles;
                report.derived_triangles += mesh_report.derived_triangles;
                remaining_triangles =
                    remaining_triangles.saturating_sub(mesh_report.derived_triangles);
                if !geometry_changed {
                    report.skipped_meshes += 1;
                    continue;
                }
                let rounded = host.assets.create_geometry(rounded);
                if let Err(error) = host.scene.set_mesh_geometry(node, rounded) {
                    diagnostics.push(error_diagnostic(
                        format!("{import_path}.edge_rounding"),
                        "import_edge_rounding_assignment_failed",
                        format!("failed to bind derived render geometry: {error}"),
                        "ensure the imported mesh node remains present during recipe construction",
                    ));
                    report.rejected_edges += mesh_report.rounded_edges;
                    report.rounded_edges = report
                        .rounded_edges
                        .saturating_sub(mesh_report.rounded_edges);
                } else {
                    report.rounded_meshes += 1;
                }
            }
            Err(error) => {
                let (code, message, help) = edge_rounding_diagnostic(error);
                diagnostics.push(error_diagnostic(
                    format!("{import_path}.edge_rounding"),
                    code,
                    message,
                    help,
                ));
                report.rejected_edges += 1;
            }
        }
    }
    Some(report)
}

fn edge_rounding_diagnostic(error: EdgeRoundingError) -> (&'static str, String, &'static str) {
    match error {
        EdgeRoundingError::UnsupportedTopology => (
            "import_edge_rounding_unsupported_topology",
            "edge rounding supports triangle meshes only".to_owned(),
            "disable edge_rounding for line geometry",
        ),
        EdgeRoundingError::DeformingMesh => (
            "import_edge_rounding_deforming_mesh",
            "edge rounding does not support skinned or morphed meshes".to_owned(),
            "disable edge_rounding for deforming meshes or provide a static product mesh",
        ),
        EdgeRoundingError::InvalidOptions => (
            "import_edge_rounding_invalid_options",
            "edge rounding controls are invalid".to_owned(),
            "validate the recipe and use finite positive radius and budget controls",
        ),
        EdgeRoundingError::NonFiniteGeometry => (
            "import_edge_rounding_non_finite_geometry",
            "edge rounding found non-finite vertex positions".to_owned(),
            "repair the source mesh positions before requesting edge rounding",
        ),
        EdgeRoundingError::DegenerateTriangles { count } => (
            "import_edge_rounding_degenerate_geometry",
            format!("edge rounding found {count} degenerate triangles"),
            "repair zero-area triangles in the source mesh",
        ),
        EdgeRoundingError::OpenMesh { boundary_edges } => (
            "import_edge_rounding_open_mesh",
            format!("edge rounding requires a closed mesh; found {boundary_edges} boundary edges"),
            "close the mesh or disable edge_rounding for this import",
        ),
        EdgeRoundingError::NonManifoldMesh { nonmanifold_edges } => (
            "import_edge_rounding_nonmanifold_mesh",
            format!(
                "edge rounding requires a two-manifold mesh; found {nonmanifold_edges} nonmanifold edges"
            ),
            "repair nonmanifold topology or disable edge_rounding for this import",
        ),
        EdgeRoundingError::InconsistentWinding { edges } => (
            "import_edge_rounding_inconsistent_winding",
            format!("edge rounding found {edges} edges with inconsistent face winding"),
            "repair source face orientation before requesting edge rounding",
        ),
        EdgeRoundingError::DerivedTriangleBudgetExceeded { required, limit } => (
            "import_edge_rounding_budget_exceeded",
            format!(
                "edge rounding requires {required} derived triangles, exceeding the explicit limit {limit}"
            ),
            "raise max_derived_triangles intentionally or reduce rounding segments",
        ),
    }
}

#[allow(clippy::too_many_arguments)]
async fn import_material_handle(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    recipe_material: &SceneRecipeImportMaterialV1,
    material_path: &str,
    texture_budget: &mut RecipeTextureBudget,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<MaterialHandle> {
    let base_color = recipe_material.base_color.as_ref().and_then(|base_color| {
        import_presentation_color(
            colors,
            base_color,
            format!("{material_path}.base_color"),
            diagnostics,
        )
    });
    if let Some(pack_ref) = recipe_material.material_pack.as_ref() {
        let resolved = match policy.resolve_import_uri(
            recipe_path,
            &pack_ref.uri,
            format!("{material_path}.material_pack.uri"),
        ) {
            Ok(resolved) => resolved,
            Err(diagnostic) => {
                diagnostics.push(*diagnostic);
                return None;
            }
        };
        let loaded = match host
            .assets
            .load_photographic_material_pack(AssetPath::from(resolved.as_str()))
            .await
        {
            Ok(loaded) => loaded,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    format!("{material_path}.material_pack.uri"),
                    "material_pack_load_failed",
                    format!("scena could not load the photographic material pack: {error}"),
                    "compile or fetch the pack with `scena materials`, then reference its scena-material-pack.json",
                ));
                return None;
            }
        };
        if let Some(expected) = pack_ref.expected_archive_sha256.as_deref()
            && !expected.eq_ignore_ascii_case(&loaded.pack().source.archive_sha256)
        {
            diagnostics.push(error_diagnostic(
                format!("{material_path}.material_pack.expected_archive_sha256"),
                "material_pack_source_sha256_mismatch",
                format!(
                    "material pack archive SHA-256 is {}, but the recipe pins {expected}",
                    loaded.pack().source.archive_sha256
                ),
                "review the source change, then update the recipe lock intentionally",
            ));
            return None;
        }
        let texture_resources = loaded.pack().maps.iter().map(|map| {
            (
                loaded.texture_resource_identity(map),
                usize::try_from(map.width)
                    .unwrap_or(usize::MAX)
                    .saturating_mul(usize::try_from(map.height).unwrap_or(usize::MAX))
                    .saturating_mul(4),
            )
        });
        if let Some(diagnostic) = texture_budget.reserve_loaded_texture_resources(
            policy,
            &format!("{material_path}.material_pack"),
            texture_resources,
        ) {
            diagnostics.push(diagnostic);
            return None;
        }
        let mut material = match host.assets.try_material(loaded.material()) {
            Ok(material) => material,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    format!("{material_path}.material_pack"),
                    "material_pack_load_failed",
                    format!("loaded material pack did not resolve its material: {error}"),
                    "recompile the material pack with the current scena version",
                ));
                return None;
            }
        };
        if let Some(color) = base_color {
            material = material.with_base_color(color);
        }
        if let Some(tile_size_m) = pack_ref.tile_size_m {
            material = material.with_photographic_surface_tile_size_m(tile_size_m as f32);
        }
        if let Some(normal_scale) = recipe_material.normal_scale {
            material = material.with_normal_scale(normal_scale as f32);
        }
        if let Some(occlusion_strength) = recipe_material.occlusion_strength {
            material = material.with_occlusion_strength(occlusion_strength as f32);
        }
        material = material.with_double_sided(recipe_material.double_sided);
        if let Some(imperfection) = recipe_material.imperfection.as_ref() {
            let decoded_bytes = [
                imperfection
                    .profile
                    .replaces_normal_texture()
                    .then_some(loaded.normal_texture()),
                Some(loaded.metallic_roughness_texture()),
            ]
            .into_iter()
            .flatten()
            .filter_map(|texture| host.assets.texture(texture)?.decoded_dimensions())
            .map(|(width, height)| {
                usize::try_from(width)
                    .unwrap_or(usize::MAX)
                    .saturating_mul(usize::try_from(height).unwrap_or(usize::MAX))
                    .saturating_mul(4)
            })
            .sum();
            if let Some(diagnostic) = texture_budget.reserve_loaded_textures(
                policy,
                &format!("{material_path}.imperfection"),
                imperfection.profile.replacement_texture_count(),
                decoded_bytes,
            ) {
                diagnostics.push(diagnostic);
                return None;
            }
            let descriptor = MaterialImperfectionDesc::new(imperfection.profile)
                .with_strength(imperfection.strength as f32)
                .with_physical_scale_m(imperfection.physical_scale_m as f32)
                .with_seed(imperfection.seed);
            material = match host
                .assets
                .composite_material_imperfection(material, descriptor)
            {
                Ok(material) => material,
                Err(error) => {
                    diagnostics.push(error_diagnostic(
                        format!("{material_path}.imperfection"),
                        "material_imperfection_generation_failed",
                        format!("failed to composite material imperfection: {error}"),
                        "use a decoded material pack with normal and roughness maps",
                    ));
                    return None;
                }
            };
        }
        return match host
            .assets
            .create_photographic_material_pack_derivative(loaded.material(), material)
        {
            Ok(material) => Some(material),
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    format!("{material_path}.material_pack"),
                    "material_pack_load_failed",
                    format!("failed to preserve material-pack resolution identity: {error}"),
                    "recompile the material pack with the current scena version",
                ));
                None
            }
        };
    }
    let mut material = if let Some(preset) = recipe_material.preset.as_deref() {
        match MaterialDesc::from_preset_name(preset, base_color) {
            Some(material) => material,
            None => {
                diagnostics.push(error_diagnostic(
                    format!("{material_path}.preset"),
                    "invalid_material_preset",
                    format!("import material preset '{preset}' is not supported"),
                    format!("use one of: {}", MaterialDesc::PRESET_NAMES.join(", ")),
                ));
                return None;
            }
        }
    } else {
        let base_color = base_color?;
        MaterialDesc::pbr_metallic_roughness(
            base_color,
            recipe_material.metallic.unwrap_or(0.0) as f32,
            recipe_material.roughness.unwrap_or(1.0) as f32,
        )
    };
    if let Some(metallic) = recipe_material.metallic {
        material = material.with_metallic_factor(metallic as f32);
    }
    if let Some(roughness) = recipe_material.roughness {
        material = material.with_roughness_factor(roughness as f32);
    }
    if let Some(normal_scale) = recipe_material.normal_scale {
        material = material.with_normal_scale(normal_scale as f32);
    }
    if let Some(occlusion_strength) = recipe_material.occlusion_strength {
        material = material.with_occlusion_strength(occlusion_strength as f32);
    }
    material = material.with_double_sided(recipe_material.double_sided);
    Some(host.assets.create_material(material))
}

#[allow(clippy::too_many_arguments)]
async fn import_material_binding_handles(
    policy: &RecipeBuildPolicy,
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipe_path: &str,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    import: &SceneRecipeImportV1,
    import_path: &str,
    texture_budget: &mut RecipeTextureBudget,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Vec<ResolvedImportMaterialBinding> {
    let mut resolved = Vec::with_capacity(import.material_bindings.len());
    for (binding_index, binding) in import.material_bindings.iter().enumerate() {
        let path = format!("{import_path}.material_bindings[{binding_index}]");
        if let Some(material) = import_material_handle(
            policy,
            host,
            recipe_path,
            colors,
            &binding.material,
            &format!("{path}.material"),
            texture_budget,
            diagnostics,
        )
        .await
        {
            resolved.push(ResolvedImportMaterialBinding {
                source_index: binding.source_material.index,
                source_name: binding.source_material.name.clone(),
                material,
                path,
            });
        }
    }
    resolved
}

fn apply_import_material_bindings(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    root_handles: &[u64],
    bindings: &[ResolvedImportMaterialBinding],
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    if bindings.is_empty() {
        return;
    }
    let mut subtree = Vec::new();
    for root_handle in root_handles {
        let root = match host.resolve_node(*root_handle) {
            Ok(root) => root,
            Err(error) => {
                diagnostics.push(scene_host_error_diagnostic(
                    "$.imports",
                    "import_presentation_failed",
                    error,
                ));
                continue;
            }
        };
        match host.scene.subtree_nodes(root) {
            Ok(nodes) => {
                for node in nodes {
                    if !subtree.contains(&node) {
                        subtree.push(node);
                    }
                }
            }
            Err(error) => diagnostics.push(scene_host_error_diagnostic(
                "$.imports",
                "import_presentation_failed",
                error.into(),
            )),
        }
    }
    if has_errors(diagnostics) {
        return;
    }

    let inspection = host.scene.inspect_with_assets(&host.assets);
    let source_nodes = inspection
        .nodes()
        .iter()
        .filter(|node| subtree.contains(&node.node()))
        .filter_map(|node| {
            let material = node.mesh_material()?;
            let source = host.assets.material_source(material)?;
            Some((
                node.node(),
                source.material_index()?,
                source.material_name().map(str::to_owned),
            ))
        })
        .collect::<Vec<_>>();

    let diagnostic_start = diagnostics.len();
    let mut assignments = Vec::<(NodeKey, MaterialHandle)>::new();
    for binding in bindings {
        let candidates = source_nodes
            .iter()
            .filter(|(_, index, _)| *index == binding.source_index)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            diagnostics.push(error_diagnostic(
                format!("{}.source_material.index", binding.path),
                "source_material_not_found",
                format!(
                    "import has no drawable source material at index {}",
                    binding.source_index
                ),
                "run `scena inspect <asset>` and copy material.source.material_index",
            ));
            continue;
        }
        if let Some(expected_name) = binding.source_name.as_deref() {
            let observed_names = candidates
                .iter()
                .map(|(_, _, name)| name.as_deref().unwrap_or("<unnamed>"))
                .collect::<BTreeSet<_>>();
            if observed_names.len() != 1 || !observed_names.contains(expected_name) {
                diagnostics.push(error_diagnostic(
                    format!("{}.source_material.name", binding.path),
                    "source_material_identity_mismatch",
                    format!(
                        "source material index {} is named {}, not '{expected_name}'",
                        binding.source_index,
                        observed_names
                            .into_iter()
                            .map(|name| format!("'{name}'"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    "review the re-exported asset and update both source material index and name intentionally",
                ));
                continue;
            }
        }
        assignments.extend(
            candidates
                .into_iter()
                .map(|(node, _, _)| (*node, binding.material)),
        );
    }
    if has_errors(&diagnostics[diagnostic_start..]) {
        return;
    }
    for (node, material) in assignments {
        if let Err(error) = host.scene.set_mesh_material(node, material) {
            diagnostics.push(error_diagnostic(
                "$.imports",
                "import_material_binding_failed",
                format!("failed to apply source material binding: {error}"),
                "check that the imported mesh nodes remain addressable",
            ));
        }
    }
}

fn import_edge_material_handle(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    import: &SceneRecipeImportV1,
    import_path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<MaterialHandle> {
    let edge = import.edge_emphasis.as_ref()?;
    if !edge.enabled {
        return None;
    }
    let base_color = match &edge.base_color {
        Some(color) => import_presentation_color(
            colors,
            color,
            format!("{import_path}.edge_emphasis.base_color"),
            diagnostics,
        )?,
        None => Color::from_srgb_u8(255, 176, 0),
    };
    let mut material = MaterialDesc::edge(base_color, edge.stroke_width_px.unwrap_or(1.75) as f32);
    if let Some(threshold) = edge.edge_angle_threshold_degrees {
        material = material.with_edge_angle_threshold_degrees(threshold as f32);
    }
    Some(host.assets.create_material(material))
}

fn import_presentation_color(
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    value: &str,
    path: String,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<Color> {
    match authored_color(colors, value) {
        Ok(color) => Some(color),
        Err(diagnostic) => {
            diagnostics.push((*diagnostic).with_path(path));
            None
        }
    }
}
