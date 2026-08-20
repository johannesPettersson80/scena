use super::{SceneHostCore, error_diagnostic, scene_host_error_diagnostic};
use crate::assets::DefaultAssetFetcher;
use crate::geometry::edge_rounding::{EdgeRoundingError, EdgeRoundingOptions, round_hard_edges};
use crate::scene::recipe::{
    SceneRecipeDiagnosticV1, SceneRecipeImportEdgeRoundingReportV1, SceneRecipeImportV1,
};

pub(super) fn apply_import_edge_rounding(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    import: &SceneRecipeImportV1,
    root_handles: &[u64],
    import_path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<SceneRecipeImportEdgeRoundingReportV1> {
    let edge = import.edge_rounding.as_ref()?;
    let mut report = SceneRecipeImportEdgeRoundingReportV1 {
        enabled: edge.enabled,
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
    if !edge.enabled {
        return Some(report);
    }
    let mut roots = Vec::with_capacity(root_handles.len());
    for handle in root_handles {
        match host.resolve_node(*handle) {
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
    let mut bounds = None;
    for root in &roots {
        match host.scene.node_world_bounds(*root, &host.assets) {
            Ok(Some(value)) => {
                bounds = Some(bounds.map_or(value, |current: crate::Aabb| current.union(value)))
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
    let Some(bounds) = bounds else {
        diagnostics.push(error_diagnostic(
            format!("{import_path}.edge_rounding"),
            "import_edge_rounding_bounds_missing",
            "edge rounding requires a bounded imported mesh",
            "supply finite static triangle geometry",
        ));
        return Some(report);
    };
    let extent = (bounds.half_extent() * 2.0).max_element().max(1.0e-6);
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
    let mut remaining = edge.max_derived_triangles;
    for (node, handle, transform) in meshes {
        report.inspected_meshes += 1;
        let geometry = match host.assets.try_geometry(handle) {
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
        let scale = transform.scale.abs().max_element().max(1.0e-6);
        let options = EdgeRoundingOptions::new(extent * edge.radius_fraction as f32 / scale)
            .with_segments(edge.segments)
            .with_edge_angle_degrees(edge.edge_angle_threshold_degrees as f32)
            .with_curve_target_error(extent / scale / 4096.0)
            .with_max_derived_triangles(remaining);
        match round_hard_edges(&geometry, options) {
            Ok((rounded, mesh)) => {
                let changed = rounded != geometry;
                report.eligible_edges += mesh.eligible_edges;
                report.rounded_edges += mesh.rounded_edges;
                report.skipped_edges += mesh.skipped_edges;
                report.rejected_edges += mesh.rejected_edges;
                report.removed_degenerate_triangles += mesh.removed_degenerate_triangles;
                report.source_triangles += mesh.source_triangles;
                report.derived_triangles += mesh.derived_triangles;
                remaining = remaining.saturating_sub(mesh.derived_triangles);
                if !changed {
                    report.skipped_meshes += 1;
                    continue;
                }
                if let Err(error) = host
                    .scene
                    .set_mesh_geometry(node, host.assets.create_geometry(rounded))
                {
                    diagnostics.push(error_diagnostic(
                        format!("{import_path}.edge_rounding"),
                        "import_edge_rounding_assignment_failed",
                        format!("failed to bind derived render geometry: {error}"),
                        "ensure the imported mesh node remains present during recipe construction",
                    ));
                    report.rejected_edges += mesh.rounded_edges;
                    report.rounded_edges = report.rounded_edges.saturating_sub(mesh.rounded_edges);
                } else {
                    report.rounded_meshes += 1;
                }
            }
            Err(error) => {
                let (code, message, help) = diagnostic(error);
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

fn diagnostic(error: EdgeRoundingError) -> (&'static str, String, &'static str) {
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
