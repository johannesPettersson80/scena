use std::collections::BTreeMap;

use super::common::DiagnosticPathExt;
use crate::GeometryHandle;
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeBuildResourceV1, SceneRecipeColorV1, SceneRecipeDiagnosticV1,
    SceneRecipeGeometryV1,
};
use crate::scene_host::SceneHostCore;

use super::super::error_diagnostic;
use super::super::policy::RecipeBuildBudget;
mod construction;
#[path = "geometry/projection.rs"]
mod projection;
use construction::authored_geometry;
use projection::projected_geometry_counts;

pub(in crate::scene_host::recipe) fn build_authored_geometries(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    recipes: &[SceneRecipeGeometryV1],
    build_budget: &mut RecipeBuildBudget,
    manifest: &mut Vec<SceneRecipeBuildResourceV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, GeometryHandle> {
    let mut handles = BTreeMap::new();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.geometries[{index}]");
        let projected = match projected_geometry_counts(recipe) {
            Ok(counts) => counts,
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path(path));
                continue;
            }
        };
        if projected.vertices > policy.max_vertices() as u64 {
            diagnostics.push(error_diagnostic(
                &path,
                "policy_violation",
                format!(
                    "geometry '{}' projects {} vertices, exceeding RecipeBuildPolicy max_vertices {}",
                    recipe.id,
                    projected.vertices,
                    policy.max_vertices()
                ),
                "reduce primitive tessellation or raise the operator-owned max_vertices policy",
            ));
            continue;
        }
        if projected.indices > policy.max_indices() as u64 {
            diagnostics.push(error_diagnostic(
                &path,
                "policy_violation",
                format!(
                    "geometry '{}' projects {} indices, exceeding RecipeBuildPolicy max_indices {}",
                    recipe.id,
                    projected.indices,
                    policy.max_indices()
                ),
                "reduce primitive tessellation or raise the operator-owned max_indices policy",
            ));
            continue;
        }
        if let Some(diagnostic) = build_budget.reserve_geometry(
            policy,
            "$.geometries",
            projected.vertices,
            projected.indices,
        ) {
            diagnostics.push(diagnostic);
            continue;
        }
        let (kind, geometry) = match authored_geometry(recipe, colors) {
            Ok(value) => value,
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path(format!("{path}.primitive")));
                continue;
            }
        };
        let vertex_count = geometry.vertices().len();
        let index_count = geometry.indices().len();
        if vertex_count > policy.max_vertices() {
            diagnostics.push(error_diagnostic(
                &path,
                "policy_violation",
                format!(
                    "geometry '{}' has {vertex_count} vertices, exceeding RecipeBuildPolicy max_vertices {}",
                    recipe.id,
                    policy.max_vertices()
                ),
                "simplify the geometry or raise the operator-owned max_vertices policy",
            ));
            continue;
        }
        if index_count > policy.max_indices() {
            diagnostics.push(error_diagnostic(
                &path,
                "policy_violation",
                format!(
                    "geometry '{}' has {index_count} indices, exceeding RecipeBuildPolicy max_indices {}",
                    recipe.id,
                    policy.max_indices()
                ),
                "simplify the geometry or raise the operator-owned max_indices policy",
            ));
            continue;
        }
        let handle = host.assets.create_geometry(geometry);
        handles.insert(recipe.id.clone(), handle);
        manifest.push(SceneRecipeBuildResourceV1 {
            id: recipe.id.clone(),
            kind,
            vertex_count: Some(vertex_count),
            index_count: Some(index_count),
        });
    }
    handles
}
