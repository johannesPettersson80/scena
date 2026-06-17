use std::collections::BTreeMap;

use crate::assets::DefaultAssetFetcher;
use crate::geometry::{GeometryMorphTarget, GeometrySkin};
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeBuildResourceV1, SceneRecipeDiagnosticV1, SceneRecipeMorphV1,
    SceneRecipeSkinV1,
};
use crate::scene_host::SceneHostCore;
use crate::{GeometryHandle, Vec3};

use super::super::error_diagnostic;

pub(in crate::scene_host::recipe) fn build_authored_morphs(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeMorphV1],
    geometries: &BTreeMap<String, GeometryHandle>,
    manifest: &mut Vec<SceneRecipeBuildResourceV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, GeometryHandle> {
    let mut handles = BTreeMap::new();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.morphs[{index}]");
        let Some(source_handle) = geometries.get(&recipe.source_geometry).copied() else {
            diagnostics.push(error_diagnostic(
                &path,
                "unknown_geometry_ref",
                format!(
                    "morph '{}' references missing source geometry '{}'",
                    recipe.id, recipe.source_geometry
                ),
                "declare the source geometry before the morph",
            ));
            continue;
        };
        let Some(source_geometry) = host.assets.geometry(source_handle) else {
            diagnostics.push(error_diagnostic(
                &path,
                "geometry_missing",
                format!(
                    "morph '{}' source geometry '{}' could not be resolved",
                    recipe.id, recipe.source_geometry
                ),
                "declare a valid source geometry before the morph",
            ));
            continue;
        };
        let targets = recipe
            .targets
            .iter()
            .map(|target| {
                GeometryMorphTarget::new(
                    target
                        .position_deltas
                        .iter()
                        .map(|delta| Vec3::new(delta[0] as f32, delta[1] as f32, delta[2] as f32))
                        .collect(),
                )
            })
            .collect::<Vec<_>>();
        let geometry = match source_geometry.with_morph_targets(targets) {
            Ok(geometry) => geometry,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "morph_create_failed",
                    format!("morph '{}' is invalid: {error:?}", recipe.id),
                    "emit one finite position delta per source vertex for every morph target",
                ));
                continue;
            }
        };
        if !passes_geometry_policy(policy, &geometry, &path, &recipe.id, diagnostics) {
            continue;
        }
        let vertex_count = geometry.vertices().len();
        let index_count = geometry.indices().len();
        let handle = host.assets.create_geometry(geometry);
        handles.insert(recipe.id.clone(), handle);
        manifest.push(SceneRecipeBuildResourceV1 {
            id: recipe.id.clone(),
            kind: "morph".to_owned(),
            vertex_count: Some(vertex_count),
            index_count: Some(index_count),
        });
    }
    handles
}

pub(in crate::scene_host::recipe) fn build_authored_skins(
    policy: &RecipeBuildPolicy,
    host: &SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeSkinV1],
    geometries: &BTreeMap<String, GeometryHandle>,
    manifest: &mut Vec<SceneRecipeBuildResourceV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> BTreeMap<String, GeometryHandle> {
    let mut handles = BTreeMap::new();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.skins[{index}]");
        let Some(source_handle) = geometries.get(&recipe.source_geometry).copied() else {
            diagnostics.push(error_diagnostic(
                &path,
                "unknown_geometry_ref",
                format!(
                    "skin '{}' references missing source geometry '{}'",
                    recipe.id, recipe.source_geometry
                ),
                "declare the source geometry before the skin",
            ));
            continue;
        };
        let Some(source_geometry) = host.assets.geometry(source_handle) else {
            diagnostics.push(error_diagnostic(
                &path,
                "geometry_missing",
                format!(
                    "skin '{}' source geometry '{}' could not be resolved",
                    recipe.id, recipe.source_geometry
                ),
                "declare a valid source geometry before the skin",
            ));
            continue;
        };
        let weights = recipe
            .weights
            .iter()
            .map(|row| [row[0] as f32, row[1] as f32, row[2] as f32, row[3] as f32])
            .collect();
        let geometry = match source_geometry.with_skin(GeometrySkin::new(
            recipe.influence_indices().to_vec(),
            weights,
        )) {
            Ok(geometry) => geometry,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "skin_create_failed",
                    format!("skin '{}' is invalid: {error:?}", recipe.id),
                    "emit one four-index and four-weight row per source vertex",
                ));
                continue;
            }
        };
        if !passes_geometry_policy(policy, &geometry, &path, &recipe.id, diagnostics) {
            continue;
        }
        let vertex_count = geometry.vertices().len();
        let index_count = geometry.indices().len();
        let handle = host.assets.create_geometry(geometry);
        handles.insert(recipe.id.clone(), handle);
        manifest.push(SceneRecipeBuildResourceV1 {
            id: recipe.id.clone(),
            kind: "skin".to_owned(),
            vertex_count: Some(vertex_count),
            index_count: Some(index_count),
        });
    }
    handles
}

fn passes_geometry_policy(
    policy: &RecipeBuildPolicy,
    geometry: &crate::GeometryDesc,
    path: &str,
    id: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> bool {
    let vertex_count = geometry.vertices().len();
    if vertex_count > policy.max_vertices() {
        diagnostics.push(error_diagnostic(
            path,
            "policy_violation",
            format!(
                "geometry '{id}' has {vertex_count} vertices, exceeding RecipeBuildPolicy max_vertices {}",
                policy.max_vertices()
            ),
            "simplify the geometry or raise the operator-owned max_vertices policy",
        ));
        return false;
    }
    let index_count = geometry.indices().len();
    if index_count > policy.max_indices() {
        diagnostics.push(error_diagnostic(
            path,
            "policy_violation",
            format!(
                "geometry '{id}' has {index_count} indices, exceeding RecipeBuildPolicy max_indices {}",
                policy.max_indices()
            ),
            "simplify the geometry or raise the operator-owned max_indices policy",
        ));
        return false;
    }
    true
}
